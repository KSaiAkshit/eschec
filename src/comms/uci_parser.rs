use tracing::warn;

#[derive(Debug, PartialEq)]
pub enum UciCommand {
    /// "uci" cmd, sent at startup
    Uci,
    /// "isready" cmd, to check if engine is ready
    IsReady,
    /// "ucinewgame" cmd, to setup a new game state
    UciNewGame,
    /// "position" cmd, to setup the board
    Position {
        startpos: bool,
        fen: Option<String>,
        moves: Vec<String>,
    },
    /// "go" cmd, to start search + time controlls
    Go(GoParams),
    /// "stop" cmd, to stop search
    Stop,
    /// "setoption" cmd, to configure engine options
    SetOption { name: String, value: String },
    /// "quit" cmd, to exit game
    Quit,
    /// unknown or unsupported cmd
    Unknown(String),
}

#[derive(Debug, PartialEq, Default)]
pub struct GoParams {
    pub wtime: Option<u64>,
    pub btime: Option<u64>,
    pub winc: Option<u64>,
    pub binc: Option<u64>,
    pub moves_to_go: Option<u64>,
    pub depth: Option<u16>,
    pub infinite: bool,
}

pub fn parse_line(line: &str) -> UciCommand {
    warn!("UCI: {line}");
    let parts: Vec<&str> = line.trim().split_ascii_whitespace().collect();
    if parts.is_empty() {
        return UciCommand::Unknown(line.to_string());
    }

    match parts[0] {
        "uci" => UciCommand::Uci,
        "isready" => UciCommand::IsReady,
        "stop" => UciCommand::Stop,
        "quit" => UciCommand::Quit,
        "position" => parse_position(&parts[1..]),
        "go" => parse_go(&parts[1..]),
        "ucinewgame" => UciCommand::UciNewGame,
        "setoption" => parse_setoption(&parts[1..]),
        _ => UciCommand::Unknown(line.to_string()),
    }
}

fn parse_position(parts: &[&str]) -> UciCommand {
    let mut fen: Option<String> = None;
    let mut moves: Vec<String> = Vec::new();
    let mut startpos = false;

    let moves_idx = parts.iter().position(|&p| p == "moves");

    let position_parts = if let Some(idx) = moves_idx {
        &parts[..idx]
    } else {
        parts
    };

    if !position_parts.is_empty() {
        if position_parts[0] == "startpos" {
            startpos = true;
        } else if position_parts[0] == "fen" {
            fen = Some(position_parts[1..].join(" "));
        }
    }

    if let Some(idx) = moves_idx {
        moves = parts[idx + 1..].iter().map(|s| s.to_string()).collect();
    }

    UciCommand::Position {
        startpos,
        fen,
        moves,
    }
}

fn parse_go(parts: &[&str]) -> UciCommand {
    let mut params = GoParams::default();

    let mut i = 0;
    while i < parts.len() {
        match parts[i] {
            "wtime" => {
                if let Some(val) = parts.get(i + 1) {
                    params.wtime = val.parse().ok();
                }
                i += 1;
            }
            "btime" => {
                if let Some(val) = parts.get(i + 1) {
                    params.btime = val.parse().ok();
                }
                i += 1;
            }
            "winc" => {
                if let Some(val) = parts.get(i + 1) {
                    params.winc = val.parse().ok();
                }
                i += 1;
            }
            "binc" => {
                if let Some(val) = parts.get(i + 1) {
                    params.binc = val.parse().ok();
                }
                i += 1;
            }
            "movestogo" => {
                if let Some(val) = parts.get(i + 1) {
                    params.moves_to_go = val.parse().ok();
                }
                i += 1;
            }
            "depth" => {
                if let Some(val) = parts.get(i + 1) {
                    params.depth = val.parse().ok();
                }
                i += 1;
            }
            "infinite" => params.infinite = true,
            _ => {}
        }
        i += 1;
    }

    UciCommand::Go(params)
}

fn parse_setoption(parts: &[&str]) -> UciCommand {
    // setoption name <name> [value <value>]
    let mut name = String::new();
    let mut value = String::new();
    let mut i = 0;
    while i < parts.len() {
        match parts[i] {
            "name" => {
                i += 1;
                while i < parts.len() && parts[i] != "value" {
                    if !name.is_empty() {
                        name.push(' ');
                    }
                    name.push_str(parts[i]);
                    i += 1;
                }
            }
            "value" => {
                i += 1;
                while i < parts.len() {
                    if !value.is_empty() {
                        value.push(' ');
                    }
                    value.push_str(parts[i]);
                    i += 1;
                }
            }
            _ => i += 1,
        }
    }
    UciCommand::SetOption { name, value }
}
