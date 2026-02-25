use ratatui::style::Color;

pub struct Event {
    pub tick: u64,
    pub message: String,
    pub color: Color,
}

pub struct EventLog {
    pub events: Vec<Event>,
    pub max_events: usize,
}

impl EventLog {
    pub fn new() -> Self {
        EventLog {
            events: Vec::new(),
            max_events: 100,
        }
    }

    pub fn log(&mut self, tick: u64, message: String, color: Color) {
        self.events.push(Event {
            tick,
            message,
            color,
        });
        if self.events.len() > self.max_events {
            self.events.remove(0);
        }
    }

    pub fn recent(&self, count: usize) -> &[Event] {
        let start = self.events.len().saturating_sub(count);
        &self.events[start..]
    }
}
