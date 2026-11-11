use crate::pane::{Pane, PaneEffect};
use crate::render::Frame;
use crate::{Key, Modifier, Modifiers, MouseButton, NamedKey, PaneBuilder, ScrollDelta};
use kurbo::Point;
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::net::{SocketAddr, TcpListener};
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender, channel};
use std::time::{Duration, Instant};

pub use crate::renderers::anyrender::Offscreen;
pub use anyrender::ImageRenderer;
#[cfg(feature = "headless-skia")]
pub use anyrender_skia::SkiaImageRenderer;
#[cfg(feature = "headless-vello")]
pub use anyrender_vello::VelloImageRenderer;
#[cfg(feature = "headless-vello-cpu")]
pub use anyrender_vello_cpu::VelloCpuImageRenderer;

const DEFAULT_SCALE: f64 = 2.;

pub enum Transport {
    Stdio,
    Tcp(SocketAddr),
}

impl Transport {
    pub fn from_env() -> Option<Self> {
        let value = std::env::var("HAVEN_HEADLESS").ok()?;
        Some(match value.as_str() {
            "stdio" => Transport::Stdio,
            other => Transport::Tcp(
                other
                    .strip_prefix("tcp:")
                    .and_then(|address| address.parse().ok())
                    .unwrap_or_else(|| {
                        panic!("HAVEN_HEADLESS must be `stdio` or `tcp:<address>`, got `{other}`")
                    }),
            ),
        })
    }
}

pub struct HeadlessApp<State> {
    state: State,
    panes: HashMap<&'static str, PaneBuilder<State>>,
}

impl<State: 'static> HeadlessApp<State> {
    pub fn new(state: State) -> Self {
        Self {
            state,
            panes: HashMap::new(),
        }
    }

    pub fn pane(mut self, pane: PaneBuilder<State>) -> Self {
        self.panes.insert(pane.name, pane);
        self
    }

    pub fn run<R: ImageRenderer>(self, transport: Transport) {
        let (messages, inbox) = channel();
        let mut runner = Runner::<State, R> {
            state: self.state,
            builders: self.panes,
            surfaces: HashMap::new(),
            current: None,
            offscreen: Offscreen::default(),
            messages: messages.clone(),
        };
        let mut at_start: Vec<_> = runner
            .builders
            .values()
            .filter_map(|pane| pane.open_at_start.then_some(pane.name))
            .collect();
        at_start.sort_unstable();
        for name in at_start {
            runner.open(name);
        }
        listen(transport, messages);
        runner.serve(inbox);
    }
}

enum Message {
    Command(String, Sender<String>),
    Wake(&'static str),
}

enum Command {
    Pane(String),
    Resize(u32, u32),
    Scale(f64),
    Move(Point),
    Press(MouseButton),
    Release(MouseButton),
    Click(Point, MouseButton),
    Drag(Point, Point),
    Scroll(ScrollDelta),
    Leave,
    Key(Key),
    KeyDown(Key),
    KeyUp(Key),
    Type(String),
    Hold(Modifiers),
    Wait(Duration),
    Snapshot(PathBuf),
    Quit,
}

struct Surface<State> {
    pane: Pane<State>,
    width: u32,
    height: u32,
    scale: f64,
    frame: Option<Frame>,
}

impl<State: 'static> Surface<State> {
    fn draw(&mut self, state: &mut State) -> Vec<PaneEffect> {
        if self.frame.is_some() {
            return Vec::new();
        }
        let (frame, effects) = self.pane.redraw(
            state,
            (self.width as f64 * self.scale).round() as u32,
            (self.height as f64 * self.scale).round() as u32,
            self.scale,
        );
        self.frame = Some(frame);
        effects
    }
}

struct Runner<State, R: ImageRenderer> {
    state: State,
    builders: HashMap<&'static str, PaneBuilder<State>>,
    surfaces: HashMap<&'static str, Surface<State>>,
    current: Option<&'static str>,
    offscreen: Offscreen<R>,
    messages: Sender<Message>,
}

impl<State: 'static, R: ImageRenderer> Runner<State, R> {
    fn serve(&mut self, inbox: Receiver<Message>) {
        while let Ok(message) = inbox.recv() {
            match message {
                Message::Wake(name) => self.wake(name),
                Message::Command(line, reply) => {
                    let command = parse(&line);
                    let quit = matches!(command, Ok(Command::Quit));
                    let result = command.and_then(|command| self.execute(command, &inbox));
                    let _ = reply.send(match result {
                        Ok(detail) if detail.is_empty() => "ok".to_string(),
                        Ok(detail) => format!("ok {detail}"),
                        Err(error) => format!("err {error}"),
                    });
                    if quit || self.surfaces.is_empty() {
                        return;
                    }
                }
            }
        }
    }

    fn open(&mut self, name: &'static str) {
        self.current = Some(name);
        if self.surfaces.contains_key(name) {
            return;
        }
        let Some(builder) = self.builders.get(name).cloned() else {
            return;
        };
        let (width, height) = builder.inner_size.unwrap_or((1044, 800));
        let mut pane = builder.build();
        let messages = self.messages.clone();
        pane.set_wake_handler(Arc::new(move || {
            let _ = messages.send(Message::Wake(name));
        }));
        let mut surface = Surface {
            pane,
            width,
            height,
            scale: DEFAULT_SCALE,
            frame: None,
        };
        let effects = surface.draw(&mut self.state);
        self.surfaces.insert(name, surface);
        self.apply(name, effects);
    }

    fn close(&mut self, name: &'static str) {
        if let Some(surface) = self.surfaces.remove(name) {
            surface.pane.close(&mut self.state);
        }
        if self.current == Some(name) {
            self.current = self.surfaces.keys().next().copied();
        }
    }

    fn apply(&mut self, name: &'static str, effects: Vec<PaneEffect>) {
        for effect in effects {
            match effect {
                PaneEffect::Open(other) => self.open(other),
                PaneEffect::Close => self.close(name),
                PaneEffect::Redraw => {
                    if let Some(surface) = self.surfaces.get_mut(name) {
                        surface.frame = None;
                    }
                }
            }
        }
    }

    fn wake(&mut self, name: &'static str) {
        let Some(surface) = self.surfaces.get_mut(name) else {
            return;
        };
        let effects = surface.pane.wake(&mut self.state);
        surface.frame = None;
        self.apply(name, effects);
    }

    fn input(
        &mut self,
        event: impl FnOnce(&mut Pane<State>, &mut State) -> Vec<PaneEffect>,
    ) -> Result<String, String> {
        let name = self.current_name()?;
        let surface = self.surfaces.get_mut(name).expect("current pane is open");
        let mut effects = surface.draw(&mut self.state);
        effects.extend(event(&mut surface.pane, &mut self.state));
        surface.frame = None;
        self.apply(name, effects);
        Ok(String::new())
    }

    fn current_name(&self) -> Result<&'static str, String> {
        self.current
            .ok_or_else(|| "no pane is open; use `pane <name>`".to_string())
    }

    fn execute(&mut self, command: Command, inbox: &Receiver<Message>) -> Result<String, String> {
        match command {
            Command::Pane(name) => {
                let name = *self
                    .builders
                    .keys()
                    .find(|known| **known == name)
                    .ok_or_else(|| format!("no pane named `{name}`"))?;
                self.open(name);
                Ok(String::new())
            }
            Command::Resize(width, height) => {
                let name = self.current_name()?;
                let surface = self.surfaces.get_mut(name).expect("current pane is open");
                surface.width = width;
                surface.height = height;
                surface.frame = None;
                Ok(String::new())
            }
            Command::Scale(scale) => {
                let name = self.current_name()?;
                self.surfaces
                    .get_mut(name)
                    .expect("current pane is open")
                    .scale = scale;
                self.input(|pane, _| pane.scale_factor_changed(scale))
            }
            Command::Move(point) => self.input(|pane, state| pane.move_to(state, point)),
            Command::Press(button) => self.input(|pane, state| pane.press_button(state, button)),
            Command::Release(button) => {
                self.input(|pane, state| pane.release_button(state, button))
            }
            Command::Click(point, button) => self.input(|pane, state| {
                let mut effects = pane.move_to(state, point);
                effects.extend(pane.press_button(state, button));
                effects.extend(pane.release_button(state, button));
                effects
            }),
            Command::Drag(from, to) => self.input(|pane, state| {
                let mut effects = pane.move_to(state, from);
                effects.extend(pane.press_button(state, MouseButton::Left));
                effects.extend(pane.move_to(state, to));
                effects.extend(pane.release_button(state, MouseButton::Left));
                effects
            }),
            Command::Scroll(delta) => self.input(|pane, state| pane.scroll(state, delta)),
            Command::Leave => self.input(|pane, state| pane.exit(state)),
            Command::Key(key) => self.input(|pane, state| {
                let mut effects = pane.key_pressed(state, key.clone());
                effects.extend(pane.key_released(state, key));
                effects
            }),
            Command::KeyDown(key) => self.input(|pane, state| pane.key_pressed(state, key)),
            Command::KeyUp(key) => self.input(|pane, state| pane.key_released(state, key)),
            Command::Type(text) => self.input(|pane, state| {
                let mut effects = Vec::new();
                for character in text.chars() {
                    let key = match character {
                        ' ' => Key::Named(NamedKey::Space),
                        other => Key::Character(other.to_string()),
                    };
                    effects.extend(pane.key_pressed(state, key.clone()));
                    effects.extend(pane.key_released(state, key));
                }
                effects
            }),
            Command::Hold(modifiers) => self.input(|pane, _| pane.modifiers_changed(modifiers)),
            Command::Wait(duration) => {
                let deadline = Instant::now() + duration;
                loop {
                    match inbox.recv_timeout(deadline.saturating_duration_since(Instant::now())) {
                        Ok(Message::Wake(name)) => self.wake(name),
                        Ok(Message::Command(_, reply)) => {
                            let _ = reply.send("err busy waiting".to_string());
                        }
                        Err(RecvTimeoutError::Timeout | RecvTimeoutError::Disconnected) => {
                            return Ok(String::new());
                        }
                    }
                }
            }
            Command::Snapshot(path) => {
                let name = self.current_name()?;
                let surface = self.surfaces.get_mut(name).expect("current pane is open");
                let effects = surface.draw(&mut self.state);
                let frame = surface.frame.as_ref().expect("drawn above");
                let png = self
                    .offscreen
                    .png(frame)
                    .map_err(|error| error.to_string())?;
                std::fs::write(&path, png)
                    .map_err(|error| format!("couldn't write {}: {error}", path.display()))?;
                let detail = format!("{} {}x{}", path.display(), frame.width, frame.height);
                self.apply(name, effects);
                Ok(detail)
            }
            Command::Quit => Ok(String::new()),
        }
    }
}

fn parse(line: &str) -> Result<Command, String> {
    let line = line.trim();
    let (word, rest) = line.split_once(' ').unwrap_or((line, ""));
    let rest = rest.trim();
    let args: Vec<&str> = rest.split_whitespace().collect();
    let number = |index: usize| -> Result<f64, String> {
        let arg = args
            .get(index)
            .ok_or_else(|| format!("`{word}` needs more arguments"))?;
        arg.parse::<f64>()
            .ok()
            .filter(|value| value.is_finite())
            .ok_or_else(|| format!("`{arg}` isn't a number"))
    };
    let size = |index: usize| -> Result<u32, String> {
        let value = number(index)?;
        (value >= 1.)
            .then_some(value.round() as u32)
            .ok_or_else(|| format!("`{value}` must be at least 1"))
    };
    let point = |index: usize| -> Result<Point, String> {
        Ok(Point::new(number(index)?, number(index + 1)?))
    };
    let button = |index: usize| {
        args.get(index)
            .map_or(Ok(MouseButton::Left), |b| mouse_button(b))
    };
    let text = || {
        (!rest.is_empty())
            .then(|| rest.to_string())
            .ok_or_else(|| format!("`{word}` needs an argument"))
    };
    Ok(match word {
        "pane" => Command::Pane(text()?),
        "resize" => Command::Resize(size(0)?, size(1)?),
        "scale" => Command::Scale(
            Some(number(0)?)
                .filter(|scale| *scale > 0.)
                .ok_or_else(|| "scale must be above 0".to_string())?,
        ),
        "move" => Command::Move(point(0)?),
        "press" => Command::Press(button(0)?),
        "release" => Command::Release(button(0)?),
        "click" => Command::Click(point(0)?, button(2)?),
        "drag" => Command::Drag(point(0)?, point(2)?),
        "scroll" => Command::Scroll(ScrollDelta {
            x: number(0)? as f32,
            y: number(1)? as f32,
        }),
        "leave" => Command::Leave,
        "key" => Command::Key(key(&text()?)?),
        "keydown" => Command::KeyDown(key(&text()?)?),
        "keyup" => Command::KeyUp(key(&text()?)?),
        "type" => Command::Type(text()?),
        "hold" => Command::Hold(Modifiers::from_pressed(
            args.iter()
                .map(|name| modifier(name))
                .collect::<Result<Vec<_>, _>>()?,
        )),
        "wait" => Command::Wait(Duration::from_millis(number(0)?.max(0.) as u64)),
        "snapshot" => Command::Snapshot(PathBuf::from(text()?)),
        "quit" => Command::Quit,
        other => return Err(format!("unknown command `{other}`")),
    })
}

fn mouse_button(name: &str) -> Result<MouseButton, String> {
    Ok(match name {
        "left" => MouseButton::Left,
        "right" => MouseButton::Right,
        "middle" => MouseButton::Middle,
        "back" => MouseButton::Back,
        "forward" => MouseButton::Forward,
        other => return Err(format!("unknown mouse button `{other}`")),
    })
}

fn modifier(name: &str) -> Result<Modifier, String> {
    Ok(match name {
        "shift" => Modifier::Shift,
        "control" => Modifier::Control,
        "alt" => Modifier::Alt,
        "super" => Modifier::Super,
        other => return Err(format!("unknown modifier `{other}`")),
    })
}

fn key(name: &str) -> Result<Key, String> {
    Ok(Key::Named(match name {
        "enter" => NamedKey::Enter,
        "escape" => NamedKey::Escape,
        "space" => NamedKey::Space,
        "backspace" => NamedKey::Backspace,
        "delete" => NamedKey::Delete,
        "left" => NamedKey::ArrowLeft,
        "right" => NamedKey::ArrowRight,
        "up" => NamedKey::ArrowUp,
        "down" => NamedKey::ArrowDown,
        "home" => NamedKey::Home,
        "end" => NamedKey::End,
        "tab" => NamedKey::Tab,
        other if other.chars().count() == 1 => return Ok(Key::Character(other.to_string())),
        other => return Err(format!("unknown key `{other}`")),
    }))
}

fn listen(transport: Transport, messages: Sender<Message>) {
    match transport {
        Transport::Stdio => {
            std::thread::spawn(move || {
                serve_client(std::io::stdin().lock(), std::io::stdout(), &messages);
                let (reply, _) = channel();
                let _ = messages.send(Message::Command("quit".to_string(), reply));
            });
        }
        Transport::Tcp(address) => {
            let listener = TcpListener::bind(address)
                .unwrap_or_else(|error| panic!("couldn't listen on {address}: {error}"));
            eprintln!(
                "haven headless listening on {}",
                listener.local_addr().unwrap_or(address)
            );
            std::thread::spawn(move || {
                for stream in listener.incoming().flatten() {
                    let Ok(reader) = stream.try_clone() else {
                        continue;
                    };
                    serve_client(BufReader::new(reader), stream, &messages);
                }
            });
        }
    }
}

fn serve_client(input: impl BufRead, mut output: impl Write, messages: &Sender<Message>) {
    for line in input.lines().map_while(Result::ok) {
        if line.trim().is_empty() {
            continue;
        }
        let (reply, response) = channel();
        if messages.send(Message::Command(line, reply)).is_err() {
            return;
        }
        let Ok(response) = response.recv() else {
            return;
        };
        if writeln!(output, "{response}")
            .and_then(|_| output.flush())
            .is_err()
        {
            return;
        }
    }
}
