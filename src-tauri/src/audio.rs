//! Captura de audio y normalización a lo que espera Whisper: mono, 16 kHz,
//! `f32`.
//!
//! La conversión (downmix y resample) es pura y se testea acá. La captura viva
//! con `cpal` necesita un micrófono, así que vive detrás de la feature `app` y
//! se verifica a mano con el ejemplo `mic_probe`.

use rubato::{
    Resampler, SincFixedIn, SincInterpolationParameters, SincInterpolationType, WindowFunction,
};

/// Tasa de muestreo que espera Whisper.
pub const TARGET_RATE: u32 = 16_000;

/// Mezcla los canales intercalados a un solo canal promediándolos.
pub fn downmix_to_mono(interleaved: &[f32], channels: usize) -> Vec<f32> {
    if channels <= 1 {
        return interleaved.to_vec();
    }
    interleaved
        .chunks(channels)
        .map(|frame| frame.iter().sum::<f32>() / frame.len() as f32)
        .collect()
}

/// Largo del filtro sinc. Cuanto más largo, mejor la respuesta en frecuencia y
/// más latencia de arranque.
const SINC_LEN: usize = 128;

/// Lleva una señal mono a 16 kHz, de una sola vez.
///
/// **Para audio en vivo hay que usar [`Resampler16k`], no esta función.** Un
/// resampler recién creado arranca con el filtro vacío y se come las primeras
/// muestras (unas `SINC_LEN/2` escaladas por la relación de tasas). Llamar a
/// esto por cada bloque del micrófono metería ese hueco en cada frontera de
/// bloque: el audio saldría entrecortado.
///
/// Si el resampler no se puede construir, devuelve el audio sin tocar:
/// transcribir con la tasa equivocada da un resultado peor, pero perder el
/// segmento entero es peor todavía.
pub fn resample_to_16k(mono: &[f32], src_rate: u32) -> Vec<f32> {
    if src_rate == TARGET_RATE || mono.is_empty() {
        return mono.to_vec();
    }
    let Some(mut resampler) = build_resampler(src_rate, mono.len()) else {
        return mono.to_vec();
    };
    process_block(&mut resampler, mono).unwrap_or_else(|| mono.to_vec())
}

/// Resample continuo para el audio que va llegando del micrófono.
///
/// Mantiene un único resampler vivo entre bloques, así el filtro sólo se
/// "carga" una vez al principio y no en cada bloque. Los bloques que llegan no
/// tienen por qué medir lo mismo que el chunk interno: lo que sobra queda
/// guardado para la próxima vez.
pub struct Resampler16k {
    chunk: usize,
    pending: Vec<f32>,
    inner: Option<SincFixedIn<f32>>,
}

impl Resampler16k {
    /// `chunk` es cuántas muestras de entrada procesa el filtro por vez. Con
    /// `src_rate == 16000` no hay nada que hacer y el audio pasa de largo.
    pub fn new(src_rate: u32, chunk: usize) -> Self {
        let inner = if src_rate == TARGET_RATE || chunk == 0 {
            None
        } else {
            build_resampler(src_rate, chunk)
        };
        Self {
            chunk,
            pending: Vec::new(),
            inner,
        }
    }

    /// Agrega un bloque y devuelve todo el audio a 16 kHz que ya se pueda
    /// entregar. Lo que no completa un chunk queda esperando.
    pub fn push(&mut self, mono: &[f32]) -> Vec<f32> {
        let Some(resampler) = self.inner.as_mut() else {
            return mono.to_vec();
        };

        self.pending.extend_from_slice(mono);
        let mut out = Vec::new();
        while self.pending.len() >= self.chunk {
            let block: Vec<f32> = self.pending.drain(..self.chunk).collect();
            if let Some(mut resampled) = process_block(resampler, &block) {
                out.append(&mut resampled);
            }
        }
        out
    }
}

/// Encadena el downmix y el resample continuo: lo que el callback de captura
/// de audio tiene que hacer con cada bloque que entrega el dispositivo.
///
/// Se separa de [`AudioCapture`] para poder testear el algoritmo (¿en qué
/// orden se combinan downmix y resample? ¿qué pasa con un solo canal?) sin
/// necesitar un micrófono real. Ver el mismo patrón en `text_injector`.
pub struct CaptureConverter {
    channels: usize,
    resampler: Resampler16k,
}

impl CaptureConverter {
    pub fn new(channels: usize, src_rate: u32, chunk: usize) -> Self {
        Self {
            channels,
            resampler: Resampler16k::new(src_rate, chunk),
        }
    }

    /// Recibe un bloque intercalado tal como lo entrega el dispositivo y
    /// devuelve el audio mono a 16 kHz que ya se pueda mandar a segmentar.
    pub fn push(&mut self, interleaved: &[f32]) -> Vec<f32> {
        let mono = downmix_to_mono(interleaved, self.channels);
        self.resampler.push(&mono)
    }
}

fn build_resampler(src_rate: u32, chunk: usize) -> Option<SincFixedIn<f32>> {
    let params = SincInterpolationParameters {
        sinc_len: SINC_LEN,
        f_cutoff: 0.95,
        oversampling_factor: 128,
        interpolation: SincInterpolationType::Linear,
        window: WindowFunction::BlackmanHarris2,
    };
    let ratio = f64::from(TARGET_RATE) / f64::from(src_rate);
    SincFixedIn::<f32>::new(ratio, 2.0, params, chunk, 1).ok()
}

fn process_block(resampler: &mut SincFixedIn<f32>, block: &[f32]) -> Option<Vec<f32>> {
    let mut out = resampler.process(&[block], None).ok()?;
    if out.is_empty() {
        return None;
    }
    Some(out.swap_remove(0))
}

#[cfg(feature = "capture")]
mod system {
    use super::CaptureConverter;
    use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
    use std::sync::mpsc::Sender;

    /// Captura en vivo del micrófono, ya convertida a mono 16 kHz.
    ///
    /// No hay tests unitarios acá: `cpal` necesita hardware real, y lo que sí
    /// se puede probar sin él — el orden downmix→resample — vive en
    /// [`CaptureConverter`] y está cubierto arriba. Esto se verifica a mano con
    /// el ejemplo `mic_probe` (ver CONTRIBUTING.md § Manual verification).
    pub struct AudioCapture {
        // Un `Stream` de cpal captura mientras exista; soltarlo (drop) para.
        // Nunca se lee directamente, pero hay que retenerlo con vida.
        _stream: cpal::Stream,
    }

    impl AudioCapture {
        /// Arranca la captura en el dispositivo `device_name` (o el de entrada
        /// por defecto si es `None`) y manda por `sink` cada bloque de audio ya
        /// convertido, a medida que el driver lo entrega.
        pub fn start(device_name: Option<&str>, sink: Sender<Vec<f32>>) -> Result<Self, String> {
            let host = cpal::default_host();
            let device = find_device(&host, device_name)?;
            let config = device.default_input_config().map_err(|e| e.to_string())?;

            if config.sample_format() != cpal::SampleFormat::F32 {
                // WASAPI en modo compartido (el default en Windows) entrega
                // f32 casi siempre; el resto de formatos necesitaría una
                // conversión de tipo que no vale la pena mientras no aparezca
                // hardware real que la necesite.
                return Err(format!(
                    "formato de muestra no soportado: {:?} (se espera F32)",
                    config.sample_format()
                ));
            }

            let channels = config.channels() as usize;
            let sample_rate = config.sample_rate();
            // ~30 ms por bloque: bajo en latencia perceptible sin llamar al
            // resampler en cada puñado de muestras. El tamaño real que entrega
            // el driver puede no coincidir — `CaptureConverter` no lo necesita.
            let chunk = ((sample_rate as usize) * 30 / 1000).max(1);
            let mut converter = CaptureConverter::new(channels, sample_rate, chunk);

            let stream = device
                .build_input_stream::<f32, _, _>(
                    &config.into(),
                    move |data: &[f32], _| {
                        let out = converter.push(data);
                        if !out.is_empty() {
                            // El otro lado (el pipeline) puede haberse ido; no
                            // hay nada que hacer salvo dejar de mandar, y el
                            // próximo callback lo va a volver a intentar.
                            let _ = sink.send(out);
                        }
                    },
                    |err| eprintln!("error de captura de audio: {err}"),
                    None,
                )
                .map_err(|e| e.to_string())?;

            stream.play().map_err(|e| e.to_string())?;

            Ok(Self { _stream: stream })
        }
    }

    fn find_device(host: &cpal::Host, name: Option<&str>) -> Result<cpal::Device, String> {
        match name {
            Some(name) => host
                .input_devices()
                .map_err(|e| e.to_string())?
                .find(|d| {
                    d.description()
                        .map(|desc| desc.name() == name)
                        .unwrap_or(false)
                })
                .ok_or_else(|| format!("dispositivo de entrada no encontrado: {name}")),
            None => host
                .default_input_device()
                .ok_or_else(|| "no hay dispositivo de entrada por defecto".to_string()),
        }
    }
}

#[cfg(feature = "capture")]
pub use system::AudioCapture;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn downmix_averages_stereo() {
        let stereo = [1.0, 0.0, 0.0, 1.0]; // L,R,L,R
        assert_eq!(downmix_to_mono(&stereo, 2), vec![0.5, 0.5]);
    }

    #[test]
    fn downmix_leaves_mono_untouched() {
        let mono = [0.1, 0.2, 0.3];
        assert_eq!(downmix_to_mono(&mono, 1), mono.to_vec());
    }

    #[test]
    fn downmix_handles_more_than_two_channels() {
        // Algunas interfaces entregan 4 o más canales.
        let quad = [1.0, 1.0, 1.0, 1.0, 0.0, 0.0, 0.0, 0.0];
        assert_eq!(downmix_to_mono(&quad, 4), vec![1.0, 0.0]);
    }

    #[test]
    fn resample_halves_length_from_32k_to_16k() {
        let src: Vec<f32> = (0..320).map(|i| i as f32).collect();
        let out = resample_to_16k(&src, 32_000);

        // La mitad de muestras, menos el arranque del filtro: un resampler
        // recién creado tiene el sinc vacío y se come unas SINC_LEN/2 muestras
        // de entrada. Eso es justamente lo que hace que no sirva para audio en
        // vivo — ahí va `Resampler16k`.
        let ideal = 160;
        let warmup = SINC_LEN / 2;
        assert!(
            out.len() <= ideal && out.len() >= ideal - warmup,
            "largo inesperado: {}",
            out.len()
        );
    }

    #[test]
    fn streaming_resampler_does_not_lose_audio_between_blocks() {
        // La prueba de fuego del resampler continuo: el arranque del filtro se
        // paga una sola vez, no en cada bloque. Con un resampler nuevo por
        // bloque faltarían ~64 muestras por bloque y el audio saldría
        // entrecortado.
        let chunk = 480; // 10 ms a 48 kHz
        let blocks = 10;
        let mut r = Resampler16k::new(48_000, chunk);

        let total: usize = (0..blocks)
            .map(|_| r.push(&vec![0.25f32; chunk]).len())
            .sum();

        let ideal = blocks * chunk / 3; // 48k -> 16k
        let warmup = SINC_LEN / 2;
        assert!(
            total <= ideal && total >= ideal - warmup,
            "se perdió audio: {total} de {ideal}"
        );
    }

    #[test]
    fn streaming_resampler_accepts_blocks_of_any_size() {
        // cpal no garantiza bloques de tamaño fijo; lo que no completa un chunk
        // interno tiene que quedar guardado, no descartado.
        let mut r = Resampler16k::new(48_000, 480);
        let mut total = 0;
        for size in [100, 37, 900, 1, 442] {
            total += r.push(&vec![0.25f32; size]).len();
        }
        let fed: usize = 100 + 37 + 900 + 1 + 442;
        assert!(total > 0);
        assert!(total <= fed / 3, "no puede inventar muestras: {total}");
    }

    #[test]
    fn streaming_resampler_at_16k_passes_audio_through() {
        let mut r = Resampler16k::new(16_000, 480);
        let block = vec![0.25f32; 100];
        assert_eq!(r.push(&block), block);
    }

    #[test]
    fn resample_at_16k_is_a_no_op() {
        let src: Vec<f32> = (0..100).map(|i| i as f32 * 0.01).collect();
        assert_eq!(resample_to_16k(&src, 16_000), src);
    }

    #[test]
    fn resample_of_nothing_is_nothing() {
        // Un bloque vacío no puede hacer explotar al resampler.
        assert!(resample_to_16k(&[], 48_000).is_empty());
    }

    #[test]
    fn capture_converter_downmixes_before_resampling() {
        // El orden importa: si `push` remuestreara el interleaved crudo en vez
        // de la versión ya mezclada a mono, saldría con el largo equivocado (el
        // doble, por los dos canales) y con valores que no corresponden a
        // ningún canal real. Un tono constante en L y otro distinto en R tiene
        // que salir como el promedio de los dos, no como ninguno de ellos.
        let mut conv = CaptureConverter::new(2, 48_000, 480);
        let mut interleaved = Vec::with_capacity(480 * 2);
        for _ in 0..480 {
            interleaved.push(1.0); // L
            interleaved.push(0.0); // R
        }

        let mut total_out = Vec::new();
        for _ in 0..10 {
            total_out.extend(conv.push(&interleaved));
        }

        assert!(!total_out.is_empty());
        let middle = &total_out[total_out.len() / 4..total_out.len() * 3 / 4];
        for sample in middle {
            assert!(
                (sample - 0.5).abs() < 0.05,
                "muestra fuera de rango tras downmix+resample: {sample}"
            );
        }
    }

    #[test]
    fn capture_converter_with_one_channel_skips_the_downmix() {
        // Con un solo canal (mic mono) `push` tiene que comportarse igual que
        // alimentar el `Resampler16k` directamente: nada de dividir por 1 ni de
        // reordenar muestras.
        let mut conv = CaptureConverter::new(1, 16_000, 480);
        let block = vec![0.25f32; 100];

        assert_eq!(conv.push(&block), block);
    }

    #[test]
    fn resample_preserves_the_signal_level() {
        // Un tono constante tiene que seguir valiendo lo mismo después de
        // remuestrear: si el resampler estuviera mal armado, acá se vería como
        // atenuación o como ruido.
        let src = vec![0.5f32; 4800]; // 100 ms a 48 kHz
        let out = resample_to_16k(&src, 48_000);
        assert!(!out.is_empty());

        // Se mira el centro: los bordes tienen el transitorio del filtro.
        let middle = &out[out.len() / 4..out.len() * 3 / 4];
        for sample in middle {
            assert!(
                (sample - 0.5).abs() < 0.05,
                "muestra fuera de rango: {sample}"
            );
        }
    }
}
