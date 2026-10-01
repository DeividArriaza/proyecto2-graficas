//! Original synthesized atmospheres and transition cues; playback is delegated
//! to an installed OS player. No audio engine or network download is required.
use std::{
    env, fs,
    io::{self, BufWriter, Write},
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    time::{SystemTime, UNIX_EPOCH},
};

const RATE: u32 = 22050;
pub const FILES: [&str; 6] = [
    "odyssey.wav",
    "galaxy.wav",
    "mario64.wav",
    "odyssey-transition.wav",
    "galaxy-transition.wav",
    "mario64-transition.wav",
];

fn tone(frequency: f32, time: f32) -> f32 {
    let phase = std::f32::consts::TAU * frequency * time;
    phase.sin() * 0.8 + (phase * 2.).sin() * 0.15 + (phase * 3.).sin() * 0.05
}

pub fn synthesize(world: usize, effect: bool) -> Vec<i16> {
    let seconds = if effect { 0.8 } else { 12.0 };
    let notes: [f32; 8] = match world % 3 {
        0 => [220., 277.18, 329.63, 415.30, 369.99, 329.63, 246.94, 277.18],
        1 => [196., 293.66, 392., 440., 369.99, 293.66, 246.94, 196.],
        _ => [261.63, 329.63, 392., 349.23, 440., 392., 293.66, 329.63],
    };
    (0..(seconds * RATE as f32) as usize)
        .map(|i| {
            let t = i as f32 / RATE as f32;
            let sample = if effect {
                let u = t / seconds;
                let envelope = (std::f32::consts::PI * u).sin().powi(2);
                match world % 3 {
                    // Continuous-phase engine ascent, stellar sparkle, pipe descent.
                    0 => {
                        envelope
                            * (tone(75., t) * 0.3
                                + (std::f32::consts::TAU * (110. * t + 260. * t * t)).sin() * 0.5)
                    }
                    1 => envelope * tone(notes[(u * 8.) as usize % 8] * 3., t) * 0.55,
                    _ => envelope * (std::f32::consts::TAU * (600. * t - 260. * t * t)).sin() * 0.6,
                }
            } else {
                let step = (t / 0.375).floor() as usize;
                let note_time = t % 0.375;
                let envelope = (note_time / 0.02).min(1.) * ((0.375 - note_time) / 0.12).min(1.);
                let fade = (t / 0.1).min(1.) * ((seconds - t) / 0.1).min(1.);
                let melody = tone(notes[step % 8], note_time) * envelope;
                let bass = tone(notes[(step / 4) % 8] * 0.5, t) * 0.16;
                let atmosphere = match world % 3 {
                    0 => melody * 0.23 + bass + tone(82.41, t) * 0.04,
                    1 => {
                        melody * 0.12
                            + tone(98., t) * 0.11
                            + tone(146.83, t) * 0.08
                            + tone(notes[(step + 2) % 8] * 2., note_time) * envelope * 0.07
                    }
                    _ => {
                        melody * 0.24
                            + bass
                            + tone(notes[step % 8] * 2., note_time) * envelope * 0.07
                    }
                };
                atmosphere * fade
            };
            (sample.clamp(-0.9, 0.9) * i16::MAX as f32) as i16
        })
        .collect()
}

fn write_wav(path: &Path, samples: &[i16]) -> io::Result<()> {
    let bytes = (samples.len() * 2) as u32;
    let mut out = BufWriter::new(fs::File::create(path)?);
    out.write_all(b"RIFF")?;
    out.write_all(&(36 + bytes).to_le_bytes())?;
    out.write_all(b"WAVEfmt ")?;
    out.write_all(&16u32.to_le_bytes())?;
    out.write_all(&1u16.to_le_bytes())?; // PCM
    out.write_all(&1u16.to_le_bytes())?; // mono
    out.write_all(&RATE.to_le_bytes())?;
    out.write_all(&(RATE * 2).to_le_bytes())?;
    out.write_all(&2u16.to_le_bytes())?;
    out.write_all(&16u16.to_le_bytes())?;
    out.write_all(b"data")?;
    out.write_all(&bytes.to_le_bytes())?;
    for sample in samples {
        out.write_all(&sample.to_le_bytes())?;
    }
    out.flush()
}

pub fn export(dir: &Path) -> io::Result<()> {
    fs::create_dir_all(dir)?;
    // Never overwrite the user's soundtrack or a previous export.
    if FILES.iter().any(|file| dir.join(file).exists()) || dir.join("manifest.json").exists() {
        return Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            "el directorio ya contiene audios o manifest; usa otro destino",
        ));
    }
    let mut entries = Vec::new();
    for (index, name) in FILES.iter().enumerate() {
        let samples = synthesize(index % 3, index >= 3);
        write_wav(&dir.join(name), &samples)?;
        let hash = samples.iter().fold(1469598103934665603u64, |h, s| {
            (h ^ u64::from(*s as u16)).wrapping_mul(1099511628211)
        });
        entries.push(format!(
            "{{\"file\":\"{name}\",\"samples\":{},\"seconds\":{},\"pcm_hash\":\"{hash:016x}\"}}",
            samples.len(),
            samples.len() as f32 / RATE as f32
        ));
    }
    fs::write(dir.join("manifest.json"), format!("{{\"source\":\"original Rust synthesis, not Nintendo recordings\",\"sample_rate\":{RATE},\"channels\":1,\"bits\":16,\"tracks\":[{}]}}\n", entries.join(",")))
}

fn stop(child: &mut Option<Child>) {
    if let Some(mut process) = child.take() {
        let _ = process.kill();
        let _ = process.wait(); // Reap only the process we launched.
    }
}

/// A short reproducible device/lifecycle check, independent of the display.
pub fn demo(custom: Option<&Path>) -> io::Result<()> {
    let mut audio = Audio::new(custom, false, 0)?;
    for world in 0..3 {
        for _ in 0..3 {
            std::thread::sleep(std::time::Duration::from_millis(100));
            audio.tick();
        }
        audio.transition(world);
        for _ in 0..8 {
            std::thread::sleep(std::time::Duration::from_millis(100));
            audio.tick();
        }
        if audio.player.is_none() {
            return Err(io::Error::other(
                "no hay salida de audio disponible para la prueba",
            ));
        }
        audio.arrive((world + 1) % 3);
    }
    println!("AUDIO_DEMO: tres ambientes y tres efectos reproducidos; ciclo cerrado, procesos detenidos al salir");
    Ok(())
}

pub struct Audio {
    dir: PathBuf,
    player: Option<PathBuf>,
    music: Option<Child>,
    effect: Option<Child>,
    world: usize,
    muted: bool,
    travelling: bool,
    temporary: bool,
}
impl Audio {
    pub fn new(custom: Option<&Path>, muted: bool, world: usize) -> io::Result<Self> {
        let temporary = custom.is_none();
        let dir = if let Some(dir) = custom {
            for name in FILES {
                if !dir.join(name).is_file() {
                    return Err(io::Error::new(
                        io::ErrorKind::NotFound,
                        format!("audio faltante: {}", dir.join(name).display()),
                    ));
                }
            }
            dir.to_path_buf()
        } else {
            let stamp = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos();
            let dir = env::temp_dir().join(format!(
                "mario-diorama-audio-{}-{stamp}",
                std::process::id()
            ));
            fs::create_dir(&dir)?;
            if let Err(error) = export(&dir) {
                // This directory and these known files were created only by us.
                for name in FILES {
                    let _ = fs::remove_file(dir.join(name));
                }
                let _ = fs::remove_file(dir.join("manifest.json"));
                let _ = fs::remove_dir(&dir);
                return Err(error);
            }
            dir
        };
        let path = env::var_os("PATH").unwrap_or_default();
        let player = ["pw-play", "aplay"].iter().find_map(|name| {
            env::split_paths(&path)
                .map(|dir| dir.join(name))
                .find(|file| file.is_file())
        });
        if player.is_none() {
            eprintln!(
                "Audio: instala pw-play (PipeWire) o aplay (ALSA); el render continúa sin sonido."
            );
        }
        let mut audio = Self {
            dir,
            player,
            music: None,
            effect: None,
            world: world % 3,
            muted,
            travelling: false,
            temporary,
        };
        audio.start_music();
        Ok(audio)
    }

    fn launch(&mut self, index: usize, effect: bool) -> Option<Child> {
        let player = self.player.as_ref()?;
        let mut command = Command::new(player);
        if player.file_name().is_some_and(|name| name == "pw-play") {
            command.args(["--volume", if effect { "0.65" } else { "0.35" }]);
        } else {
            command.arg("-q");
        }
        match command
            .arg(self.dir.join(FILES[index]))
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
        {
            Ok(child) => Some(child),
            Err(error) => {
                eprintln!("Audio no disponible: {error}");
                self.player = None;
                None
            }
        }
    }
    fn start_music(&mut self) {
        if !self.muted && !self.travelling && self.music.is_none() {
            self.music = self.launch(self.world, false);
        }
    }
    pub fn transition(&mut self, from: usize) {
        self.travelling = true;
        stop(&mut self.music);
        stop(&mut self.effect);
        if !self.muted {
            self.effect = self.launch(from % 3 + 3, true);
        }
    }
    pub fn arrive(&mut self, world: usize) {
        self.world = world % 3;
        self.travelling = false;
        stop(&mut self.effect);
        self.start_music();
    }
    pub fn toggle(&mut self) {
        self.muted = !self.muted;
        if self.muted {
            stop(&mut self.music);
            stop(&mut self.effect);
        } else {
            self.start_music();
        }
    }
    pub fn tick(&mut self) {
        if let Some(child) = self.music.as_mut() {
            match child.try_wait() {
                Ok(Some(status)) => {
                    self.music = None;
                    if !status.success() {
                        eprintln!("Audio: el reproductor no pudo abrir la salida; render activo sin sonido.");
                        self.player = None;
                    }
                }
                Ok(None) => {}
                Err(_) => {
                    stop(&mut self.music);
                    self.player = None;
                }
            }
        }
        if let Some(child) = self.effect.as_mut() {
            match child.try_wait() {
                Ok(Some(status)) => {
                    self.effect = None;
                    if !status.success() {
                        self.player = None;
                    }
                }
                Ok(None) => {}
                Err(_) => {
                    stop(&mut self.effect);
                    self.player = None;
                }
            }
        }
        self.start_music();
    }
    pub fn label(&self) -> &'static str {
        if self.muted {
            "B:AUDIO SILENCIADO"
        } else if self.player.is_none() {
            "B:AUDIO NO DISPONIBLE"
        } else if self.travelling {
            "B:AUDIO EFECTO DE VIAJE"
        } else {
            "B:AUDIO AMBIENTE"
        }
    }
}
impl Drop for Audio {
    fn drop(&mut self) {
        stop(&mut self.music);
        stop(&mut self.effect);
        if self.temporary {
            for name in FILES {
                let _ = fs::remove_file(self.dir.join(name));
            }
            let _ = fs::remove_file(self.dir.join("manifest.json"));
            let _ = fs::remove_dir(&self.dir);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn muted_travel_preserves_state_and_cleans_only_owned_audio() {
        let mut audio = Audio::new(None, true, 0).unwrap();
        audio.player = None; // No device or player is needed by this test.
        let owned = audio.dir.clone();
        assert!(owned.join("galaxy.wav").exists());
        assert!(export(&owned).is_err()); // Never overwrite existing tracks.
        audio.transition(0);
        assert!(audio.travelling);
        assert!(audio.music.is_none() && audio.effect.is_none());
        audio.arrive(1);
        assert_eq!(audio.world, 1);
        assert!(!audio.travelling);
        audio.toggle();
        audio.tick();
        assert_eq!(audio.label(), "B:AUDIO NO DISPONIBLE");
        drop(audio);
        assert!(!owned.exists());
    }
    #[test]
    fn tracks_are_original_distinct_bounded_and_effects_match_transition() {
        let mut hashes = std::collections::HashSet::new();
        for world in 0..3 {
            for effect in [false, true] {
                let samples = synthesize(world, effect);
                assert_eq!(samples.len(), if effect { 17640 } else { 264600 });
                assert_eq!(samples[0], 0);
                assert!(samples.iter().any(|s| s.unsigned_abs() > 1000));
                assert!(samples.iter().all(|s| s.unsigned_abs() < 30000));
                let hash = samples.iter().fold(0u64, |h, s| {
                    h.wrapping_mul(31).wrapping_add(*s as u16 as u64)
                });
                assert!(hashes.insert(hash));
            }
        }
    }
}
