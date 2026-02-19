use std::time::Instant;

#[derive(Debug)]
pub struct PulseTimer {
    start: Instant,
    opacity: f32,
    active: bool,
}

impl PulseTimer {
    pub fn new(cx: &mut gpui::Context<Self>) -> Self {
        cx.spawn(async move |this, cx| {
            loop {
                cx.background_executor()
                    .timer(std::time::Duration::from_millis(33))
                    .await;
                let should_continue = cx.update(|cx| {
                    if let Some(timer) = this.upgrade() {
                        timer.update(cx, |t, cx| {
                            if t.active {
                                t.tick();
                                cx.notify();
                            }
                        });
                        true
                    } else {
                        false
                    }
                });
                if !should_continue {
                    break;
                }
            }
        })
        .detach();

        Self {
            start: Instant::now(),
            opacity: 1.0,
            active: false,
        }
    }

    pub fn set_active(&mut self, active: bool) {
        if active && !self.active {
            self.start = Instant::now();
        }
        self.active = active;
    }

    #[must_use]
    pub fn is_active(&self) -> bool {
        self.active
    }

    fn tick(&mut self) {
        let t = self.start.elapsed().as_secs_f32();
        let sin_val = (t * std::f32::consts::TAU / 1.5).sin();
        self.opacity = 0.4 + 0.6 * f32::midpoint(sin_val, 1.0);
    }

    #[must_use]
    pub fn opacity(&self) -> f32 {
        self.opacity
    }
}
