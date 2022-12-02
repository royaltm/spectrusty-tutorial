/*
    This program is free to use under the terms of the Blue Oak Model License 1.0.0.
    See: https://blueoakcouncil.org/license/1.0.0
*/
//! This is an example implementation of STEP 1 of the SPECTRUSTY tutorial using `minifb` framebuffer.
//!
//! See: https://github.com/royaltm/spectrusty-tutorial/
use core::mem;
use minifb::{Key, KeyRepeat, Scale, Window, WindowOptions, Menu};
use rand::prelude::*;

use spectrusty::z80emu::{Cpu, Z80NMOS};
use spectrusty::chip::{ControlUnit, HostConfig, MemoryAccess, ThreadSyncTimer, ula::UlaPAL};
use spectrusty::memory::{ZxMemory, Memory16k, Memory48k};
use spectrusty::video::{
    Video, Palette, PixelBuffer, BorderSize, BorderColor, 
    pixel::{PixelBufP32, SpectrumPalA8R8G8B8}
};
use spectrusty::peripherals::{KeyboardInterface, ZXKeyboardMap};

use spectrusty_utils::keyboard::minifb::update_keymap;

type Result<T> = core::result::Result<T, Box<dyn std::error::Error>>;

#[derive(Default)]
struct ZxSpectrum<C: Cpu, M: ZxMemory> {
    cpu: C,
    ula: UlaPAL<M>
}

// Let's create some sugar definitions
type ZxSpectrum16k<C> = ZxSpectrum<C, Memory16k>;
type ZxSpectrum48k<C> = ZxSpectrum<C, Memory48k>;

enum ZxSpectrumModel<C: Cpu> {
    Spectrum16(ZxSpectrum16k<C>),
    Spectrum48(ZxSpectrum48k<C>),
}

#[derive(Debug, Clone, Copy)]
enum Action {
    ChangeModel(ModelReq),
    Exit
}

#[derive(Debug, Clone, Copy)]
enum ModelReq {
    Spectrum16,
    Spectrum48,
}

struct Env<'a> {
    window: &'a mut Window,
    width: usize,
    height: usize,
    border: BorderSize,
    pixels: &'a mut Vec<u32>
}

// the type of PixelBuffer
type PixelBuf<'a> = PixelBufP32<'a>;
// the type of PixelBuffer::Pixel
type Pixel<'a> = <PixelBuf<'a> as PixelBuffer<'a>>::Pixel;
// the palette used
type SpectrumPal = SpectrumPalA8R8G8B8;

impl<C: Cpu, M: ZxMemory> ZxSpectrum<C, M> {
    fn update_keyboard<F: FnOnce(ZXKeyboardMap) -> ZXKeyboardMap>(
            &mut self,
            update_keys: F)
    {
        let keymap = update_keys( self.ula.get_key_state() );
        self.ula.set_key_state(keymap);
    }

    fn run_frame(&mut self) {
        self.ula.execute_next_frame(&mut self.cpu);
    }
    // `buffer` is a mutable slice of bytes.
    // `pitch` is the number of bytes of the single row of pixels.
    // `border` determines the size of the rendered screen.
    fn render_video<'a, P: Palette<Pixel=Pixel<'a>>>(
        &mut self,
        buffer: &mut [u8],
        pitch: usize,
        border: BorderSize)
    {
        self.ula.render_video_frame::<PixelBuf, P>(buffer, pitch, border);
    }
    // so we can reset our Spectrum
    fn reset(&mut self, hard: bool) {
        self.ula.reset(&mut self.cpu, hard)
    }
    // so we can trigger Non-Maskable Interrupt
    fn trigger_nmi(&mut self) -> bool {
        self.ula.nmi(&mut self.cpu)
    }
}

impl<C: Cpu> ZxSpectrumModel<C> {
    fn into_cpu(self) -> C {
        match self {
            ZxSpectrumModel::Spectrum16(spec16) => spec16.cpu,
            ZxSpectrumModel::Spectrum48(spec48) => spec48.cpu,
        }        
    }
    fn as_mem_ref(&self) -> &[u8] {
        match self {
            ZxSpectrumModel::Spectrum16(spec16) => spec16.ula.memory_ref().mem_ref(),
            ZxSpectrumModel::Spectrum48(spec48) => spec48.ula.memory_ref().mem_ref(),
        }
    }
    fn border_color(&self) -> BorderColor  {
        match self {
            ZxSpectrumModel::Spectrum16(spec16) => spec16.ula.border_color(),
            ZxSpectrumModel::Spectrum48(spec48) => spec48.ula.border_color(),
        }
    }
    // hot-swap hardware models
    fn change_model(self, request: ModelReq) -> Self {
        use ZxSpectrumModel::*;
        match (&self, request) {
            (Spectrum16(..), ModelReq::Spectrum16)|
            (Spectrum48(..), ModelReq::Spectrum48) => return self,
            _ => {}
        }
        let mem = self.as_mem_ref();
        let border = self.border_color();
        match request {
            ModelReq::Spectrum16 => {
                let mut spec16 = ZxSpectrum16k::<C>::default();
                let mem16 = spec16.ula.memory_mut().mem_mut();
                let len = mem16.len().min(mem.len());
                mem16[..len].copy_from_slice(&mem[..len]);
                spec16.cpu = self.into_cpu();
                spec16.ula.set_border_color(border);
                Spectrum16(spec16)
            }
            ModelReq::Spectrum48 => {
                let mut spec48 = ZxSpectrum48k::<C>::default();
                let mem48 = spec48.ula.memory_mut().mem_mut();
                let len = mem48.len().min(mem.len());
                mem48[..len].copy_from_slice(&mem[..len]);
                spec48.cpu = self.into_cpu();
                spec48.ula.set_border_color(border);
                Spectrum48(spec48)
            }
        }
    }
}

const MENU_EXIT_ID:       usize = 0;
const MENU_HARD_RESET_ID: usize = 1;
const MENU_SOFT_RESET_ID: usize = 2;
const MENU_TRIG_NMI_ID:   usize = 3;
const MENU_MODEL_16_ID:   usize = 4;
const MENU_MODEL_48_ID:   usize = 5;

fn open_window(title: &str, width: usize, height: usize) -> Result<Window> {
    let mut winopt = WindowOptions::default();
    winopt.scale = Scale::X2;
    let mut window = Window::new(&title, width, height, winopt)
                            .map_err(|e| e.to_string())?;
    window.limit_update_rate(None);

    let mut menu = Menu::new("Main").map_err(|e| e.to_string())?;
    let mut models = Menu::new("Models").map_err(|e| e.to_string())?;

    models.add_item("ZX Spectrum 16k", MENU_MODEL_16_ID)
        .shortcut(Key::F1, minifb::MENU_KEY_CTRL)
        .build();
    models.add_item("ZX Spectrum 48k", MENU_MODEL_48_ID)
        .shortcut(Key::F2, minifb::MENU_KEY_CTRL)
        .build();

    menu.add_item("Hard reset", MENU_HARD_RESET_ID)
        .shortcut(Key::F1, 0)
        .build();
    menu.add_item("Soft reset", MENU_SOFT_RESET_ID)
        .shortcut(Key::F2, 0)
        .build();
    menu.add_item("Trigger NMI", MENU_TRIG_NMI_ID)
        .shortcut(Key::F3, 0)
        .build();
    menu.add_sub_menu("Select model", &models);
    menu.add_item("Exit", MENU_EXIT_ID)
        .shortcut(Key::F10, 0)
        .build();

    window.add_menu(&menu);

    Ok(window)
}

fn update_keymap_from_window_events(window: &Window, mut cur: ZXKeyboardMap) -> ZXKeyboardMap {
    let shift_dn = window.is_key_down(Key::LeftShift) || window.is_key_down(Key::RightShift);
    let ctrl_dn = window.is_key_down(Key::LeftCtrl) || window.is_key_down(Key::RightCtrl);
    for k in window.get_keys_pressed(KeyRepeat::No) {
        cur = update_keymap(cur, k, true, shift_dn, ctrl_dn);
    }
    for k in window.get_keys_released() {
        cur = update_keymap(cur, k, false, shift_dn, ctrl_dn);
    }
    cur
}

// transform the frame buffer to the format needed by render_video
fn acquire_video_buffer(pixels: &mut [u32], pixel_width: usize) -> (&mut [u8], usize) {
    let pitch = pixel_width * mem::size_of::<u32>();
    let (_, buffer, _) = unsafe { pixels.align_to_mut::<u8>() };
    (buffer, pitch)
}

fn run<C: Cpu, M: ZxMemory>(
        spectrum: &mut ZxSpectrum<C, M>,
        Env { window, width, height, border, pixels }: Env<'_>,
    ) -> Result<Action>
{
    let title = format!("ZX Spectrum {}k", spectrum.ula.memory_ref().ram_ref().len() / 1024);
    window.set_title(&title);

    let mut sync = ThreadSyncTimer::new(UlaPAL::<M>::frame_duration_nanos());
    let mut synchronize_frame = || {
        if let Err(missed) = sync.synchronize_thread_to_frame() {
            println!("*** paused for: {} frames ***", missed);
        }
    };

    let is_running = |window: &Window| -> bool {
        window.is_open() && !window.is_key_down(Key::Escape)
    };

    // emulator main loop
    while is_running(window) {
        spectrum.update_keyboard(|keys| update_keymap_from_window_events(window, keys));

        spectrum.run_frame();

        let (video_buffer, pitch) = acquire_video_buffer(pixels.as_mut(), width);
        spectrum.render_video::<SpectrumPal>(video_buffer, pitch, border);

        // update_display
        window.update_with_buffer(&pixels, width, height)
              .map_err(|e| e.to_string())?;

        if let Some(menu_id) = window.is_menu_pressed() {
            match menu_id {
                MENU_HARD_RESET_ID  => spectrum.reset(true),
                MENU_SOFT_RESET_ID  => spectrum.reset(false),
                MENU_TRIG_NMI_ID    => { spectrum.trigger_nmi(); },
                MENU_MODEL_16_ID    => return Ok(Action::ChangeModel(ModelReq::Spectrum16)),
                MENU_MODEL_48_ID    => return Ok(Action::ChangeModel(ModelReq::Spectrum48)),
                MENU_EXIT_ID        => break,
                _ => {}
            }
        }

        synchronize_frame();
    }

    Ok(Action::Exit)
}

fn main() -> Result<()> {
    simple_logger::SimpleLogger::new().with_level(log::LevelFilter::Info).init()?;
    // parsing the first command argument as a size of the border
    let border: BorderSize = if let Some(arg) = std::env::args().nth(1) {
        arg.parse()?
    }
    else {
        BorderSize::Full
    };
    // build the hardware
    let mut spec16 = ZxSpectrum16k::<Z80NMOS>::default();
    // some entropy in memory for nice visuals
    spec16.ula.memory_mut().fill_mem(.., random)?;
    // get the software
    let rom_file = std::fs::File::open("resources/roms/48.rom")?;
    // put the software into the hardware
    spec16.ula.memory_mut().load_into_rom(rom_file)?;

    // width and height of the rendered frame image area in pixels
    let (width, height) = <UlaPAL<Memory16k> as Video>::render_size_pixels(border);
    // more convenient for minifb
    let (width, height) = (width as usize, height as usize);
    // minifb uses u32 XRGB pixels
    let mut pixels: Vec<u32> = vec![0; width * height];
    // open window
    let mut window = open_window("ZX Spectrum", width, height)?;

    let mut spectrum = ZxSpectrumModel::Spectrum16(spec16);

    loop {
        use ZxSpectrumModel::*;
        let env = Env { window: &mut window, width, height, border, pixels: &mut pixels };
        let req = match &mut spectrum {
            Spectrum16(spec16) => run(spec16, env)?,
            Spectrum48(spec48) => run(spec48, env)?
        };

        spectrum = match req {
            Action::ChangeModel(spec) => spectrum.change_model(spec),
            Action::Exit => break
        };
    }

    Ok(())
}
