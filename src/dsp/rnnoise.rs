use nnnoiseless::DenoiseState;
use ringbuf::{
    HeapRb,
    traits::{Consumer, Producer, Split, Observer},
};

pub struct NeuralNoiseSuppressor {
    enabled: bool,
    denoise_state_l: Box<DenoiseState<'static>>,
    denoise_state_r: Box<DenoiseState<'static>>,
    in_buf_l: ringbuf::HeapProd<f32>,
    in_cons_l: ringbuf::HeapCons<f32>,
    out_buf_l: ringbuf::HeapProd<f32>,
    out_cons_l: ringbuf::HeapCons<f32>,

    in_buf_r: ringbuf::HeapProd<f32>,
    in_cons_r: ringbuf::HeapCons<f32>,
    out_buf_r: ringbuf::HeapProd<f32>,
    out_cons_r: ringbuf::HeapCons<f32>,
}

impl NeuralNoiseSuppressor {
    pub fn new() -> Self {
        let rb_in_l = HeapRb::new(4800);
        let rb_out_l = HeapRb::new(4800);
        let rb_in_r = HeapRb::new(4800);
        let rb_out_r = HeapRb::new(4800);
        let (in_buf_l, in_cons_l) = rb_in_l.split();
        let (out_buf_l, out_cons_l) = rb_out_l.split();
        let (in_buf_r, in_cons_r) = rb_in_r.split();
        let (out_buf_r, out_cons_r) = rb_out_r.split();

        Self {
            enabled: false,
            denoise_state_l: DenoiseState::new(),
            denoise_state_r: DenoiseState::new(),
            in_buf_l,
            in_cons_l,
            out_buf_l,
            out_cons_l,
            in_buf_r,
            in_cons_r,
            out_buf_r,
            out_cons_r,
        }
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    pub fn process(&mut self, l: f32, r: f32) -> (f32, f32) {
        if !self.enabled {
            return (l, r);
        }

        let _ = self.in_buf_l.try_push(l);
        let _ = self.in_buf_r.try_push(r);

        while self.in_cons_l.occupied_len() >= 480 {
            let mut chunk_l = [0.0; 480];
            let mut chunk_r = [0.0; 480];

            for i in 0..480 {
                chunk_l[i] = self.in_cons_l.try_pop().unwrap_or(0.0);
                chunk_r[i] = self.in_cons_r.try_pop().unwrap_or(0.0);
            }

            let mut out_chunk_l = [0.0; 480];
            let mut out_chunk_r = [0.0; 480];
            self.denoise_state_l.process_frame(&mut out_chunk_l, &chunk_l);
            self.denoise_state_r.process_frame(&mut out_chunk_r, &chunk_r);

            for i in 0..480 {
                let _ = self.out_buf_l.try_push(out_chunk_l[i]);
                let _ = self.out_buf_r.try_push(out_chunk_r[i]);
            }
        }

        let out_l = self.out_cons_l.try_pop().unwrap_or(l);
        let out_r = self.out_cons_r.try_pop().unwrap_or(r);

        (out_l, out_r)
    }
}
