use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::Stream;
use ringbuf::traits::Producer;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};

pub struct WasapiCapture {
    pub is_running: Arc<AtomicBool>,
    pub active_sink_name: Arc<Mutex<String>>,
    _stream: Option<Stream>,
}

impl WasapiCapture {
    pub fn start_auto_capture(mut producer: ringbuf::HeapProd<f32>) -> Option<Self> {
        let is_running = Arc::new(AtomicBool::new(true));
        let active_sink_name = Arc::new(Mutex::new("Windows WASAPI Loopback".to_string()));

        let host = cpal::default_host();
        let device = host.default_output_device()?;
        let config = device.default_output_config().ok()?;

        let err_fn = |err| eprintln!("an error occurred on stream: {}", err);

        let stream = match config.sample_format() {
            cpal::SampleFormat::F32 => device
                .build_input_stream(
                    &config.into(),
                    move |data: &[f32], _: &_| {
                        for &sample in data {
                            let _ = producer.try_push(sample);
                        }
                    },
                    err_fn,
                    None,
                )
                .ok(),
            _ => None,
        };

        if let Some(ref s) = stream {
            let _ = s.play();
        }

        Some(Self {
            is_running,
            active_sink_name,
            _stream: stream,
        })
    }
}
