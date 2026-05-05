#[derive(Default)]
pub(super) struct Osc7Parser {
    state: Osc7State,
    payload: Vec<u8>,
}

#[derive(Default)]
enum Osc7State {
    #[default]
    Ground,
    Esc,
    OscStart,
    Osc7Semicolon,
    OscIgnore,
    Osc7Payload,
    Osc7Esc,
}

impl Osc7Parser {
    const MAX_PAYLOAD_LEN: usize = 4096;

    pub(super) fn advance(&mut self, bytes: &[u8]) -> Vec<String> {
        let mut events = Vec::new();

        for &byte in bytes {
            match self.state {
                Osc7State::Ground => {
                    if byte == 0x1b {
                        self.state = Osc7State::Esc;
                    }
                }
                Osc7State::Esc => {
                    self.state = match byte {
                        b']' => Osc7State::OscStart,
                        0x1b => Osc7State::Esc,
                        _ => Osc7State::Ground,
                    };
                }
                Osc7State::OscStart => {
                    self.state = match byte {
                        b'7' => Osc7State::Osc7Semicolon,
                        0x07 => Osc7State::Ground,
                        0x1b => Osc7State::OscIgnore,
                        _ => Osc7State::OscIgnore,
                    };
                }
                Osc7State::Osc7Semicolon => {
                    self.state = match byte {
                        b';' => {
                            self.payload.clear();
                            Osc7State::Osc7Payload
                        }
                        0x07 => Osc7State::Ground,
                        0x1b => Osc7State::OscIgnore,
                        _ => Osc7State::OscIgnore,
                    };
                }
                Osc7State::OscIgnore => {
                    self.state = match byte {
                        0x07 => Osc7State::Ground,
                        0x1b => Osc7State::Esc,
                        _ => Osc7State::OscIgnore,
                    };
                }
                Osc7State::Osc7Payload => match byte {
                    0x07 => {
                        self.finish(&mut events);
                    }
                    0x1b => {
                        self.state = Osc7State::Osc7Esc;
                    }
                    _ => self.push_payload_byte(byte),
                },
                Osc7State::Osc7Esc => {
                    if byte == b'\\' {
                        self.finish(&mut events);
                    } else {
                        self.push_payload_byte(0x1b);
                        self.push_payload_byte(byte);
                        self.state = Osc7State::Osc7Payload;
                    }
                }
            }
        }

        events
    }

    fn push_payload_byte(&mut self, byte: u8) {
        if self.payload.len() < Self::MAX_PAYLOAD_LEN {
            self.payload.push(byte);
        } else {
            self.payload.clear();
            self.state = Osc7State::OscIgnore;
        }
    }

    fn finish(&mut self, events: &mut Vec<String>) {
        if let Some(cwd) = parse_osc7_working_directory(&self.payload) {
            events.push(cwd);
        }
        self.payload.clear();
        self.state = Osc7State::Ground;
    }
}

fn parse_osc7_working_directory(payload: &[u8]) -> Option<String> {
    let payload = std::str::from_utf8(payload).ok()?;
    let rest = payload.strip_prefix("file://")?;
    let path = if rest.starts_with('/') {
        rest
    } else {
        let path_start = rest.find('/')?;
        &rest[path_start..]
    };

    percent_decode(path.as_bytes())
}

fn percent_decode(input: &[u8]) -> Option<String> {
    let mut decoded = Vec::with_capacity(input.len());
    let mut i = 0;

    while i < input.len() {
        if input[i] == b'%' {
            let hi = *input.get(i + 1)?;
            let lo = *input.get(i + 2)?;
            decoded.push((hex_value(hi)? << 4) | hex_value(lo)?);
            i += 3;
        } else {
            decoded.push(input[i]);
            i += 1;
        }
    }

    String::from_utf8(decoded).ok()
}

fn hex_value(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}
