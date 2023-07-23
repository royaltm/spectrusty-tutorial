/*
    This program is free to use under the terms of the Blue Oak Model License 1.0.0.
    See: https://blueoakcouncil.org/license/1.0.0
*/
//! This is the example implementation of STEP 3 of the SPECTRUSTY tutorial using `minifb` framebuffer
//! and the `cpal` audio layer.
//!
//! See: https://github.com/royaltm/spectrusty-tutorial/
use core::convert::TryFrom;
use core::fmt::Write;
use core::mem;
use std::path::Path;
use std::fs::{File, OpenOptions};
use minifb::{Key, KeyRepeat, Scale, Window, WindowOptions, Menu};
use rand::prelude::*;
#[allow(unused_imports)]
use log::{error, warn, info, debug, trace};
use spectrusty_tutorial::{*, menus::AppMenu};

use spectrusty::audio::{
    AudioSample, EarMicAmps4, EarOutAmps4, EarInAmps2,
    Blep, AudioFrame, FromSample, EarMicOutAudioFrame, EarInAudioFrame,
    synth::{BandLimited, ext::BandLimitedExt},
    carousel::AudioFrameResult,
    host::cpal::AudioHandleAnyFormat
};
use spectrusty::z80emu::{Cpu, Z80NMOS};
use spectrusty::clock::FTs;
use spectrusty::chip::{
    ControlUnit, FrameState, HostConfig, MemoryAccess, MicOut, EarIn,
    ThreadSyncTimer,
    ula::UlaPAL
};
use spectrusty::memory::{ZxMemory, Memory16k, Memory48k};
use spectrusty::video::{
    Video, Palette, PixelBuffer, BorderSize, BorderColor,
    pixel::{PixelBufP32, SpectrumPalA8R8G8B8}
};
use spectrusty::peripherals::{KeyboardInterface, ZXKeyboardMap};
use spectrusty::formats::tap::{read_tap_pulse_iter, TapChunkRead, TapChunkInfo};

use spectrusty_utils::{
    tap::{Tape, Tap},
    keyboard::minifb::update_keymap
};

type Result<T> = core::result::Result<T, Box<dyn std::error::Error>>;

#[derive(Default)]
struct ZxSpectrum<C: Cpu, M: ZxMemory> {
    cpu: C,
    ula: UlaPAL<M>,
    nmi_request: bool,
    // the state of the emulator program
    state: EmulatorState
}

#[derive(Default)]
struct EmulatorState {
    // the TAPE recorder, maybe a tape is inside?
    tape: Tape<File>,
    // a record of a previous frame EAR IN counter
    prev_ear_in_counter: u32,
    // is the emulation paused?
    paused: bool,
    // do we want to run as fast as possible?
    turbo: bool,
    // do we want to auto accelerate and enable auto load?
    flash_tape: bool,
    // do we want to hear the tape signal?
    audible_tape: bool,
}

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ModelReq {
    Spectrum16,
    Spectrum48,
}

// the type of the audio handle
type Audio = AudioHandleAnyFormat;
// the type of the Blep implementation amplitude delta
type BlepDelta = f32; // i16, i32, f32, f64
// the type of the Blep implementation
type BandLim = BandLimited<BlepDelta>;
// the audio carousel latency
const AUDIO_LATENCY: usize = 2;

struct Env<'a> {
    window: &'a mut Window,
    width: usize,
    height: usize,
    border: BorderSize,
    pixels: &'a mut Vec<u32>,
    audio: &'a mut Audio,
    blep: &'a mut BandLim
}

// the type of PixelBuffer
type PixelBuf<'a> = PixelBufP32<'a>;
// the type of PixelBuffer::Pixel
type Pixel<'a> = <PixelBuf<'a> as PixelBuffer<'a>>::Pixel;
// the palette used
type SpectrumPal = SpectrumPalA8R8G8B8;

impl<C: Cpu, M: ZxMemory> ZxSpectrum<C, M> {
    fn info(&mut self) -> Result<String> {
        let mut info = format!("ZX Spectrum {}k", self.ula.memory_ref().ram_ref().len() / 1024);
        if self.state.paused {
            info.push_str(" ⏸ ");
        }
        else if self.state.turbo {
            info.push_str(" 🏎️ ");
        }
        // is the TAPE running?
        let running = self.state.tape.running;
        // is there any TAPE inserted at all?
        if let Some(tap) = self.state.tape.tap.as_mut() {
            let flash = if self.state.flash_tape { '⚡' } else { ' ' };
            // we'll show if the TAP sound is audible
            let audible = if self.state.audible_tape { '🔊' } else { '🔈' };
            match tap {
                Tap::Reader(..) if running => write!(info, " 🖭{}{} ⏵", flash, audible)?,
                Tap::Writer(..) if running => write!(info, " 🖭{}{} ⏺", flash, audible)?,
                tap => {
                    // The TAPE is paused so we'll show some TAP block metadata.
                    let mut rd = tap.try_reader_mut()?;
                    // `rd` when dropped will restore underlying file cursor position,
                    // so it's perfectly save to use it to read the metadata of
                    // the current chunk.
                    let chunk_no = rd.rewind_chunk()?;
                    let chunk_info = TapChunkInfo::try_from(rd.get_mut())?;
                    // restore cursor position
                    rd.done()?;
                    write!(info, " 🖭{}{} {}: {}", flash, audible, chunk_no, chunk_info)?;
                }
            }
        }
        Ok(info)
    }

    fn update_keyboard<F: FnOnce(ZXKeyboardMap) -> ZXKeyboardMap>(
            &mut self,
            update_keys: F)
    {
        let keymap = update_keys( self.ula.get_key_state() );
        self.ula.set_key_state(keymap);
    }

    // returns `Ok(is_recording)`
    fn record_tape_from_mic_out(&mut self) -> Result<bool> {
        // get the writer if the tape is inserted and is being recorded
        if let Some(ref mut writer) = self.state.tape.recording_writer_mut() {
            // extract the MIC OUT state changes as a pulse iterator
            let pulses_iter = self.ula.mic_out_pulse_iter();
            // decode the pulses as TAPE data and write it as a TAP chunk fragment
            let chunks = writer.write_pulses_as_tap_chunks(pulses_iter)?;
            if chunks != 0 {
                info!("Saved: {} TAP chunks", chunks);
            }
            if self.state.turbo || self.state.flash_tape  {
                // is the state of the pulse decoder idle?
                self.state.turbo = !writer.get_ref().is_idle();
            }
            return Ok(true)
        }
        Ok(false)
    }

    // very simple heuristics for detecting if spectrum needs some TAPE data
    fn auto_detect_load_from_tape(&mut self) -> Result<()> {
        let count = self.ula.read_ear_in_count();
        if count != 0 {
            // if turbo is on and the tape is playing
            if self.state.turbo && self.state.tape.is_playing() {
                const IDLE_THRESHOLD: u32 = 20;
                // stop the tape and slow down
                // if the EAR IN probing falls below the threshold
                if self.state.prev_ear_in_counter + count < IDLE_THRESHOLD {
                    self.state.tape.stop();
                    self.state.turbo = false;
                }
            }
            // if flash loading is enabled and a tape isn't running
            else if self.state.flash_tape && self.state.tape.is_inserted() &&
                   !self.state.tape.running {
                const PROBE_THRESHOLD: u32 = 1000;
                // play the tape and speed up
                // if the EAR IN probing exceeds the threshold
                if count > PROBE_THRESHOLD {
                    self.state.tape.play()?;
                    self.state.turbo = true;
                }
            }
            self.state.prev_ear_in_counter = count;
        }
        Ok(())
    }

    // returns `Ok(end_of_tape)`
    fn feed_ear_in_or_stop_tape(&mut self) -> Result<bool> {
        // get the reader if the tape is inserted and is being played
        if let Some(ref mut feeder) = self.state.tape.playing_reader_mut() {
            // check if any pulse is still left in the feeder
            let mut feeder = feeder.peekable();
            if feeder.peek().is_some() {
                // feed EAR IN line with pulses from our pulse iterator
                // only up to the end of a single frame
                self.ula.feed_ear_in(&mut feeder, Some(1));
            }
            else {
                // end of tape
                self.state.tape.stop();
                // always end turbo mode when the tape stops
                self.state.turbo = false;
                return Ok(true)
            }
        }
        Ok(false)
    }

    fn run_frame(&mut self) -> Result<(FTs, bool)> {
        // for tracking an effective change
        let (turbo, running) = (self.state.turbo, self.state.tape.running);

        if !self.record_tape_from_mic_out()? &&
                (self.state.flash_tape || self.state.turbo) {
            self.auto_detect_load_from_tape()?;
        }
        // clean up the internal buffers of ULA so we won't append the EAR IN data
        // to the previous frame's data
        self.ula.ensure_next_frame();
        // and we also need the timestamp of the beginning of a frame
        let fts_start = self.ula.current_tstate();

        if self.feed_ear_in_or_stop_tape()? && running {
            // only report it when the tape was running before
            info!("Auto STOP: End of TAPE");
        }

        if self.nmi_request && self.ula.nmi(&mut self.cpu) {
            // clear nmi_request only if the triggering succeeded
            self.nmi_request = false;
        }
        self.ula.execute_next_frame(&mut self.cpu);

        let fts_delta = self.ula.current_tstate() - fts_start;
        let state_changed = running != self.state.tape.running ||
                            turbo   != self.state.turbo;
        Ok((fts_delta, state_changed))
    }

    // run frames as fast as possible until a single frame duration passes
    // in real-time or if the turbo state ends automatically
    fn run_frames_accelerated(
            &mut self,
            time_sync: &mut ThreadSyncTimer
        ) -> Result<(FTs, bool)>
    {
        let mut sum: FTs = 0;
        let mut state_changed = false;
        while time_sync.check_frame_elapsed().is_none() {
            let (cycles, schg) = self.run_frame()?;
            sum += cycles;
            if schg {
                state_changed = true;
                if !self.state.turbo {
                    break;
                }
            }
        }
        Ok((sum, state_changed))
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
    // adds pulse steps to the `blep` and returns the number of samples ready to be produced.
    fn render_audio<B: Blep<SampleDelta=BlepDelta>>(&mut self, blep: &mut B) -> usize {
        // (1) add some amplitude steps to the BLEP that correspond to the EAR/MIC line changes
        if self.state.audible_tape {
            // render both EAR/MIC OUT channel
            self.ula.render_earmic_out_audio_frame::<EarMicAmps4<BlepDelta>>(blep, 0);
            // and the EAR IN channel
            self.ula.render_ear_in_audio_frame::<EarInAmps2<BlepDelta>>(blep, 0);
        }
        else {
            // render only EAR OUT channel
            self.ula.render_earmic_out_audio_frame::<EarOutAmps4<BlepDelta>>(blep, 0);
        }
        // (2) finalize the BLEP frame
        self.ula.end_audio_frame(blep)
    }
    // so we can reset our Spectrum
    fn reset(&mut self, hard: bool) {
        self.ula.reset(&mut self.cpu, hard)
    }
    // so we can trigger Non-Maskable Interrupt
    fn trigger_nmi(&mut self) {
        self.nmi_request = true;
    }

    // insert a tape file by file path
    fn insert_tape<P: AsRef<Path>>(&mut self, file_path: P) -> Result<()> {
        info!("Inserting TAP file: {}", file_path.as_ref().display());
        // open the .tap file for reading and writing, allow creating a new file
        let tap_file = OpenOptions::new()
        .read(true).write(true).create(true)
        .open(&file_path)
        .or_else(|err| {
            // if that fails, re-try for reading only
            warn!("Couldn't open TAP for writing: {:?}", err);
            OpenOptions::new().read(true).open(file_path)
        })?;
        // wrap the file into the TapChunkPulseIter
        let iter_pulse = read_tap_pulse_iter(tap_file);
        self.state.tape.tap.replace(Tap::Reader(iter_pulse));
        // or instead we could just write:
        // self.tape.insert_as_reader(tap_file);
        self.state.audible_tape = true;
        self.state.flash_tape = true;
        Ok(())
    }

    // open the file dialog and insert a selected tape file
    fn open_tape(&mut self) {
        if let Some(file_path) = open_tape_dialog() {
            if let Err(err) = self.insert_tape(&file_path) {
                error!("Error opening TAP file: {} {}", file_path.display(), err);
            }
        }
    }

    // open the save file dialog and insert a selected tape file
    fn save_tape(&mut self) {
        if let Some(file_path) = save_tape_dialog() {
            if let Err(err) = self.insert_tape(&file_path) {
                error!("Error creating TAP file: {} {}", file_path.display(), err);
            }
        }
    }

    fn update_on_user_request(&mut self, menu_id: usize) -> Result<Option<Action>> {
        match menu_id {
            MENU_EXIT_ID         => return Ok(Some(Action::Exit)),
            MENU_MODEL_16_ID     => return Ok(Some(Action::ChangeModel(ModelReq::Spectrum16))),
            MENU_MODEL_48_ID     => return Ok(Some(Action::ChangeModel(ModelReq::Spectrum48))),
            MENU_HARD_RESET_ID   => self.reset(true),
            MENU_SOFT_RESET_ID   => self.reset(false),
            MENU_TRIG_NMI_ID     => { self.trigger_nmi(); }
            MENU_TURBO_ID        => { self.state.turbo = !self.state.turbo; }
            MENU_PAUSE_ID        => { self.state.paused = true; }
            MENU_TAPE_REWIND_ID  => { self.state.tape.rewind_nth_chunk(1)?; }
            MENU_TAPE_PLAY_ID    => { self.state.tape.play()?; }
            MENU_TAPE_RECORD_ID  => { self.state.tape.record()?; }
            MENU_TAPE_STOP_ID    => { self.state.tape.stop(); }
            MENU_TAPE_PREV_ID    => { self.state.tape.rewind_prev_chunk()?; }
            MENU_TAPE_NEXT_ID    => { self.state.tape.forward_chunk()?; }
            MENU_TAPE_AUDIBLE_ID => { self.state.audible_tape = !self.state.audible_tape; }
            MENU_TAPE_FLASH_ID   => { self.state.flash_tape = !self.state.flash_tape; }
            MENU_TAPE_OPEN_ID    => { self.open_tape(); }
            MENU_TAPE_SAVE_ID    => { self.save_tape(); }
            MENU_TAPE_EJECT_ID   => { self.state.tape.eject(); }
            _ => {}
        }
        Ok(None)
    }
}

impl<C: Cpu, M: ZxMemory> From<ZxSpectrumModel<C>> for ZxSpectrum<C, M>
    where ZxSpectrum<C, M>: Default
{
    fn from(model: ZxSpectrumModel<C>) -> Self {
        let border = model.border_color();
        let other_mem = model.as_mem_ref();
        let mut spectrum = ZxSpectrum::<C, M>::default();
        let my_mem = spectrum.ula.memory_mut().mem_mut();
        let len = other_mem.len().min(my_mem.len());
        my_mem[..len].copy_from_slice(&other_mem[..len]);
        let (cpu, state) = model.into_cpu_and_state();
        spectrum.cpu = cpu;
        spectrum.state = state;
        spectrum.ula.set_border_color(border);
        spectrum
    }
}

impl<C: Cpu> ZxSpectrumModel<C> {
    fn into_cpu_and_state(self) -> (C, EmulatorState) {
        match self {
            ZxSpectrumModel::Spectrum16(spec16) => (spec16.cpu, spec16.state),
            ZxSpectrumModel::Spectrum48(spec48) => (spec48.cpu, spec48.state),
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
        match request {
            ModelReq::Spectrum16 => Spectrum16(ZxSpectrum16k::<C>::from(self)),
            ModelReq::Spectrum48 => Spectrum48(ZxSpectrum48k::<C>::from(self))
        }
    }
}

const MENU_EXIT_ID:         usize = 0;
const MENU_HARD_RESET_ID:   usize = 1;
const MENU_SOFT_RESET_ID:   usize = 2;
const MENU_TRIG_NMI_ID:     usize = 3;
const MENU_MODEL_16_ID:     usize = 4;
const MENU_MODEL_48_ID:     usize = 5;
const MENU_PAUSE_ID:        usize = 6;
const MENU_TURBO_ID:        usize = 7;
const MENU_TAPE_REWIND_ID:  usize = 100;
const MENU_TAPE_PLAY_ID:    usize = 101;
const MENU_TAPE_RECORD_ID:  usize = 102;
const MENU_TAPE_STOP_ID:    usize = 103;
const MENU_TAPE_PREV_ID:    usize = 104;
const MENU_TAPE_NEXT_ID:    usize = 105;
const MENU_TAPE_AUDIBLE_ID: usize = 106;
const MENU_TAPE_FLASH_ID:   usize = 107;
const MENU_TAPE_OPEN_ID:    usize = 108;
const MENU_TAPE_SAVE_ID:    usize = 109;
const MENU_TAPE_EJECT_ID:   usize = 110;

fn open_window(title: &str, width: usize, height: usize) -> Result<Window> {
    let mut winopt = WindowOptions::default();
    winopt.scale = Scale::X2;
    let mut window = Window::new(&title, width, height, winopt)
                            .map_err(|e| e.to_string())?;
    window.limit_update_rate(None);

    let mut menu = Menu::new("Spectrum").map_err(|e| e.to_string())?;
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
    menu.add_item("Toggle Turbo", MENU_TURBO_ID)
        .shortcut(Key::ScrollLock, 0)
        .build();
    menu.add_item("Toggle Pause", MENU_PAUSE_ID)
        .shortcut(Key::Pause, 0)
        .build();
    menu.add_sub_menu("Select model", &models);
    menu.add_item("Exit", MENU_EXIT_ID)
        .shortcut(Key::F10, 0)
        .build();

    let mut tape = Menu::new("Tape").map_err(|e| e.to_string())?;
    tape.add_item("Open a TAPE file", MENU_TAPE_OPEN_ID)
        .shortcut(Key::Insert, 0)
        .build();
    tape.add_item("Create a new TAPE file", MENU_TAPE_SAVE_ID)
        .shortcut(Key::Insert, minifb::MENU_KEY_SHIFT)
        .build();
    tape.add_item("Rewind tape", MENU_TAPE_REWIND_ID)
        .shortcut(Key::Home, 0)
        .build();
    tape.add_item("Prev chunk", MENU_TAPE_PREV_ID)
        .shortcut(Key::PageUp, 0)
        .build();
    tape.add_item("Next chunk", MENU_TAPE_NEXT_ID)
        .shortcut(Key::PageDown, 0)
        .build();
    tape.add_item("Play", MENU_TAPE_PLAY_ID)
        .shortcut(Key::F5, 0)
        .build();
    tape.add_item("Stop", MENU_TAPE_STOP_ID)
        .shortcut(Key::F6, 0)
        .build();
    tape.add_item("Record", MENU_TAPE_RECORD_ID)
        .shortcut(Key::F7, 0)
        .build();
    tape.add_item("Eject TAPE", MENU_TAPE_EJECT_ID)
        .shortcut(Key::Delete, minifb::MENU_KEY_SHIFT)
        .build();
    tape.add_item("Toggle audible", MENU_TAPE_AUDIBLE_ID)
        .shortcut(Key::F8, 0)
        .build();
    tape.add_item("Toggle flash load/save", MENU_TAPE_FLASH_ID)
        .shortcut(Key::F8, minifb::MENU_KEY_SHIFT)
        .build();

    window.add_menu(&menu);
    window.add_menu(&tape);

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

fn produce_audio_frame<T: AudioSample + FromSample<BlepDelta>>(
        output_channels: usize,
        outbuf: &mut Vec<T>,
        blep: &mut BandLim,
    )
{
    // the number of samples generated this frame
    let frame_sample_count = blep.num_samples_ended_frame().unwrap();
    // ensure the size of the audio frame buffer is exactly as we need it
    outbuf.resize(frame_sample_count * output_channels, T::silence());
    // render samples to audio channels
    blep.render_audio_fill_interleaved(outbuf, output_channels, 0);
}

fn produce_and_send_audio_frame(
        audio: &mut AudioHandleAnyFormat,
        blep: &mut BandLim
    ) -> AudioFrameResult<()>
{
    let channels = audio.channels().into();
    match audio {
        AudioHandleAnyFormat::I8(audio) =>
            audio.producer.render_frame(|out| produce_audio_frame(channels, out, blep)),
        AudioHandleAnyFormat::U8(audio) =>
            audio.producer.render_frame(|out| produce_audio_frame(channels, out, blep)),
        AudioHandleAnyFormat::I16(audio) =>
            audio.producer.render_frame(|out| produce_audio_frame(channels, out, blep)),
        AudioHandleAnyFormat::U16(audio) =>
            audio.producer.render_frame(|out| produce_audio_frame(channels, out, blep)),
        AudioHandleAnyFormat::I32(audio) =>
            audio.producer.render_frame(|out| produce_audio_frame(channels, out, blep)),
        AudioHandleAnyFormat::U32(audio) =>
            audio.producer.render_frame(|out| produce_audio_frame(channels, out, blep)),
        AudioHandleAnyFormat::I64(audio) =>
            audio.producer.render_frame(|out| produce_audio_frame(channels, out, blep)),
        AudioHandleAnyFormat::U64(audio) =>
            audio.producer.render_frame(|out| produce_audio_frame(channels, out, blep)),
        AudioHandleAnyFormat::F32(audio) =>
            audio.producer.render_frame(|out| produce_audio_frame(channels, out, blep)),
        AudioHandleAnyFormat::F64(audio) =>
            audio.producer.render_frame(|out| produce_audio_frame(channels, out, blep)),
        _ => panic!("Unsupported sample format!")
    }
    // send the frame buffer to the consumer
    audio.send_frame()
}

#[cfg(feature = "measure_cpu_freq")]
use spectrusty::video::VideoFrame;

fn run<C: Cpu, M: ZxMemory>(
        spectrum: &mut ZxSpectrum<C, M>,
        Env { window, width, height, border, pixels, audio, blep }: Env<'_>,
    ) -> Result<Action>
{
    window.set_title(&spectrum.info()?);

    let app_menu = AppMenu::new(&window);

    // ensure the Blep implementation is prepared for pulses
    spectrum.ula.ensure_audio_frame_time(blep, audio.sample_rate(), UlaPAL::<M>::CPU_HZ as f64);
    audio.play()?;

    let mut sync = ThreadSyncTimer::new(UlaPAL::<M>::frame_duration_nanos());
    fn synchronize_frame(sync: &mut ThreadSyncTimer) {
        if let Err(missed) = sync.synchronize_thread_to_frame() {
            debug!("*** paused for: {} frames ***", missed);
        }
    }

    fn is_running(window: &Window) -> bool {
        window.is_open() && !window.is_key_down(Key::Escape)
    }

    #[cfg(feature = "measure_cpu_freq")]
    measure_ticks_start!(time, dur, ticks, spectrum, UlaPAL<M>);

    // emulator main loop
    'main: while is_running(window) {
        spectrum.update_keyboard(|keys| update_keymap_from_window_events(window, keys));

        let (_, mut state_changed) = if spectrum.state.paused {
            window.limit_update_rate(Some(std::time::Duration::from_millis(100)));
            loop {
                if !is_running(window) { break 'main; }
                match app_menu.is_menu_pressed(window) {
                    Some(MENU_PAUSE_ID) => { break; }
                    Some(MENU_EXIT_ID) => { break 'main; }
                    _ => {}
                }
                window.update();
            }
            spectrum.state.paused = false;
            window.limit_update_rate(None);
            sync.restart();
            (0, true)
        } else if spectrum.state.turbo {
            spectrum.run_frames_accelerated(&mut sync)?
        }
        else {
            spectrum.run_frame()?
        };

        #[cfg(feature = "measure_cpu_freq")]
        measure_ticks!(time, dur, ticks, spectrum, UlaPAL<M>);

        let (video_buffer, pitch) = acquire_video_buffer(pixels.as_mut(), width);
        spectrum.render_video::<SpectrumPal>(video_buffer, pitch, border);

        // update_display
        window.update_with_buffer(&pixels, width, height)
              .map_err(|e| e.to_string())?;

        if let Some(menu) = app_menu.is_menu_pressed(window) {
            match spectrum.update_on_user_request(menu)? {
                Some(action) => return Ok(action),
                None => { state_changed = true; }
            }
        }

        if state_changed {
            if spectrum.state.turbo || spectrum.state.paused {
                // we won't be rendering audio when in TURBO mode or when PAUSED
                audio.pause()?;
            }
            else {
                // we need to make sure audio thread plays before we send the audio buffer
                // otherwise this thread will hang forever waiting for the response
                audio.play()?;
            }
            window.set_title(&spectrum.info()?);
        }

        if !spectrum.state.turbo && !spectrum.state.paused {
            // no audio in TURBO mode or when PAUSED
            spectrum.render_audio(blep);
            // (3) render the BLEP frame as audio samples
            produce_and_send_audio_frame(audio, blep)?;
            // (4) prepare the BLEP for the next frame.
            blep.next_frame();
        }

        if !spectrum.state.turbo {
            synchronize_frame(&mut sync);
        }
    }

    Ok(Action::Exit)
}

fn show_help() -> Result<()> {
    eprintln!("{}: [-16|48] [-b BORDER] [TAPFILE]",
        std::env::args().next().as_deref().unwrap_or("step3"));
    Ok(())
}

fn main() -> Result<()> {
    simple_logger::SimpleLogger::new().with_level(log::LevelFilter::Info).init()?;
    spectrusty_tutorial::set_dpi_awareness()?;
    let mut args = std::env::args().skip(1);
    let mut border = BorderSize::Full;
    let mut model = ModelReq::Spectrum16;
    let mut tap_file_name = None;
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "-16" =>  { model = ModelReq::Spectrum16; },
            "-48" =>  { model = ModelReq::Spectrum48; },
            "-b" => match args.next() {
                Some(arg) => { border = arg.parse()?; },
                None => return show_help(),
            },
            x if x == "" || x.starts_with("-") => return show_help(),
            // parsing the 1st command argument as path to the TAP file
            name => {
                tap_file_name = Some(name.to_string());
                break;
            }
        };
    }
    // build the hardware
    let mut spec16 = ZxSpectrum16k::<Z80NMOS>::default();
    // some entropy in memory for nice visuals
    spec16.ula.memory_mut().fill_mem(.., random)?;
    // get the software
    let rom_file = std::fs::File::open("resources/roms/48.rom")?;
    // put the software into the hardware
    spec16.ula.memory_mut().load_into_rom(rom_file)?;
    // if the user provided us with the file name
    if let Some(file_name) = tap_file_name {
        spec16.insert_tape(file_name)?;
    }

    // width and height of the rendered frame image area in pixels
    let (width, height) = <UlaPAL<Memory16k> as Video>::render_size_pixels(border);
    // more convenient for minifb
    let (width, height) = (width as usize, height as usize);
    // minifb uses u32 XRGB pixels
    let mut pixels: Vec<u32> = vec![0; width * height];
    // open window
    let mut window = open_window("ZX Spectrum", width, height)?;

    // initialize audio
    let frame_duration_nanos = <UlaPAL<Memory16k> as HostConfig>::frame_duration_nanos();
    // first the audio handle with the embedded carousel
    let mut audio = Audio::create(&cpal::default_host(), frame_duration_nanos, AUDIO_LATENCY)?;
    // second the Bandwidth-Limited Pulse Buffer implementation
    let mut blep = BandLim::new(1);

    let mut spectrum = ZxSpectrumModel::Spectrum16(spec16);

    if model != ModelReq::Spectrum16 {
        spectrum = spectrum.change_model(model);
    }

    loop {
        use ZxSpectrumModel::*;
        let env = Env { width, height, border,
                        window: &mut window, 
                        pixels: &mut pixels,
                        audio: &mut audio,
                        blep: &mut blep };

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
