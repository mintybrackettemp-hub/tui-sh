use std::io::stdout;
use std::process::Command;
use std::collections::HashMap;
use std::path::PathBuf;
use std::fs;
use crossterm::event::{self, Event, KeyCode};
use crossterm::terminal::{enable_raw_mode, disable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen};
use crossterm::execute;
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use ratatui::layout::{Layout, Constraint, Direction};
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph, ListState, Clear};
use ratatui::style::{Style, Color, Modifier};
use serde::{Deserialize, Serialize};

#[derive(Clone)]
struct Alias {
    name: String,
    command: String,
    keybind: Option<char>,
}

#[derive(Serialize, Deserialize)]
struct AliasEntry {
    command: String,
    keybind: Option<String>,
}

#[derive(Serialize, Deserialize)]
struct ConfigFile {
    aliases: HashMap<String, AliasEntry>,
    #[serde(rename = "default-shell")]
    default_shell: String,
}

enum UiMode {
    Main,
    Adding { step: u8, name: String, command: String, keybind: Option<char> },
    EditingSelect,
    Editing { index: usize, command: String },
    RemovingSelect,
    Message(String),
}

enum Focus {
    Aliases,
    Actions,
}

const MIN_W: u16 = 40;
const MIN_H: u16 = 10;

fn config_path() -> PathBuf {
    if let Some(mut d) = dirs::config_dir() {
        d.push("tuish");
        fs::create_dir_all(&d).ok();
        d.push("cnfg.json");
        d
    } else {
        PathBuf::from("./cnfg.json")
    }
}

fn write_config(path: &PathBuf, aliases: &Vec<Alias>, default_shell: &str) {
    let mut map = HashMap::new();
    for a in aliases.iter() {
        map.insert(a.name.clone(), AliasEntry { command: a.command.clone(), keybind: a.keybind.map(|c| c.to_string()) });
    }
    let cfg = ConfigFile { aliases: map, default_shell: default_shell.to_string() };
    if let Ok(s) = serde_json::to_string_pretty(&cfg) {
        let _ = fs::write(path, s);
    }
}

fn ensure_config(path: &PathBuf) -> ConfigFile {
    if !path.exists() {
        // create empty aliases by default
        let example: HashMap<String, AliasEntry> = HashMap::new();
        let cfg = ConfigFile { aliases: example, default_shell: "/bin/bash".to_string() };
        if let Ok(s) = serde_json::to_string_pretty(&cfg) {
            let _ = fs::write(path, s);
        }
        cfg
    } else {
        let data = fs::read_to_string(path).unwrap_or_default();
        serde_json::from_str(&data).unwrap_or(ConfigFile { aliases: HashMap::new(), default_shell: std::env::var("SHELL").unwrap_or_else(|_| "sh".into()) })
    }
}

fn run_shell_command_with_shell(cmd: &str, shell: &str) {
    // Leave TUI and run the command in the shell, then wait for a keypress
    disable_raw_mode().ok();
    execute!(std::io::stdout(), LeaveAlternateScreen).ok();

    let status = Command::new(shell).arg("-c").arg(cmd).status();
    match status {
        Ok(s) => println!("Command exited with: {}", s),
        Err(e) => println!("Failed to run command: {}", e),
    }

    println!("Press any key to return to the menu...");

    // Wait for one key press
    enable_raw_mode().ok();
    let _ = event::read();
}

fn main() {
    enable_raw_mode().unwrap();

    let cfg_path = config_path();
    let cfg = ensure_config(&cfg_path);

    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen).unwrap();
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend).unwrap();

    // Load aliases from config
    let mut aliases: Vec<Alias> = cfg.aliases.iter().map(|(name, entry)| Alias {
        name: name.clone(),
        command: entry.command.clone(),
        keybind: entry.keybind.as_ref().and_then(|s| s.chars().next()),
    }).collect();

    let default_shell = cfg.default_shell.clone();

    let options = vec!["Add an alias", "Edit an alias", "Remove an alias", "Go to shell", "Quit shell"];
    let mut opt_state = ListState::default();
    opt_state.select(Some(0));

    // selection state for aliases list and focus
    let mut alias_state = ListState::default();
    if !aliases.is_empty() { alias_state.select(Some(0)); } else { alias_state.select(None); }
    let mut focus = Focus::Actions;

    let mut ui_mode = UiMode::Main;
    let mut selected_opt: usize = 0;

    loop {
        // Draw UI
        terminal.draw(|f| {
            let size = f.size();

            // detect too small
            if size.width < MIN_W || size.height < MIN_H {
                let area = ratatui::layout::Rect::new(2, 2, (size.width.saturating_sub(4)).max(1), 3);
                let msg = format!("Terminal too small — need at least {}x{}", MIN_W, MIN_H);
                let p = Paragraph::new(msg).style(Style::default().fg(Color::Red).add_modifier(Modifier::BOLD));
                f.render_widget(Clear, area);
                f.render_widget(p, area);
                return;
            }

            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .margin(1)
                .constraints([
                    Constraint::Length(1), // header
                    Constraint::Min(3),     // aliases (will be clipped if too large)
                    Constraint::Length(7),  // actions
                ].as_ref())
                .split(size);

            let header = Paragraph::new("tuish").style(Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD));
            f.render_widget(header, chunks[0]);

            // Aliases block (clipped if too many) - make it selectable when focused
            let alias_items: Vec<ListItem> = if aliases.is_empty() {
                vec![ListItem::new("(no aliases)").style(Style::default().fg(Color::DarkGray))]
            } else {
                aliases.iter().map(|a| {
                    let kb = match a.keybind { Some(c) => format!(" [{}]", c), None => "".into() };
                    ListItem::new(format!("{}{} - {}", a.name, kb, a.command)).style(Style::default().fg(Color::Cyan))
                }).collect()
            };
            let mut alias_list = List::new(alias_items)
                .block(Block::default().borders(Borders::ALL).title("Aliases"));
            // highlight style only when aliases have focus
            alias_list = alias_list.highlight_style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)).highlight_symbol("-> ");
            f.render_stateful_widget(alias_list, chunks[1], &mut alias_state);

            // Options
            let opt_items: Vec<ListItem> = options.iter().map(|o| ListItem::new(o.to_string()).style(Style::default().fg(Color::White))).collect();
            let opt_list = List::new(opt_items)
                .block(Block::default().borders(Borders::ALL).title("Actions").style(Style::default().fg(Color::Green)))
                .highlight_style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
                .highlight_symbol("> ");
            f.render_stateful_widget(opt_list, chunks[2], &mut opt_state);

            // If in adding/editing mode, show a small popup
            match &ui_mode {
                UiMode::Main => { /* nothing to draw on top */ }
                UiMode::Adding { step, name, command, keybind } => {
                    let area = ratatui::layout::Rect::new(size.width/6, size.height/3, size.width*2/3, 7);
                    let mut text = vec![format!("Step {}", step)];
                    if *step == 1 { text.push(format!("Name: {}", name)); }
                    if *step == 2 { text.push(format!("Command: {}", command)); }
                    if *step == 3 { text.push(format!("Keybind (single char, or empty): {}", keybind.map(|c| c.to_string()).unwrap_or_default())); }
                    let p = Paragraph::new(text.join("\n")).block(Block::default().borders(Borders::ALL).title("Add alias"));
                    f.render_widget(Clear, area);
                    f.render_widget(p, area);
                }
                UiMode::Editing { index, command } => {
                    let area = ratatui::layout::Rect::new(size.width/6, size.height/3, size.width*2/3, 5);
                    let title = format!("Edit command for: {}", aliases.get(*index).map(|a| a.name.clone()).unwrap_or_default());
                    let p = Paragraph::new(command.clone()).block(Block::default().borders(Borders::ALL).title(title));
                    f.render_widget(Clear, area);
                    f.render_widget(p, area);
                }
                UiMode::EditingSelect => {
                    // use alias_state so selection is shared and list auto-scrolls when too long
                    let area_height = (size.height / 3).max(3);
                    let area = ratatui::layout::Rect::new(size.width/6, size.height/3, size.width*2/3, area_height);
                    let items: Vec<ListItem> = aliases.iter().map(|a| ListItem::new(format!("{} - {}", a.name, a.command))).collect();
                    let mut sel_state = ListState::default();
                    sel_state.select(alias_state.selected());
                    let list = List::new(items).block(Block::default().borders(Borders::ALL).title("Select alias to edit"))
                        .highlight_style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)).highlight_symbol("> ");
                    f.render_stateful_widget(list, area, &mut sel_state);
                }
                UiMode::RemovingSelect => {
                    let area_height = (size.height / 3).max(3);
                    let area = ratatui::layout::Rect::new(size.width/6, size.height/3, size.width*2/3, area_height);
                    let items: Vec<ListItem> = aliases.iter().map(|a| ListItem::new(format!("{} - {}", a.name, a.command))).collect();
                    let mut sel_state = ListState::default();
                    sel_state.select(alias_state.selected());
                    let list = List::new(items).block(Block::default().borders(Borders::ALL).title("Select alias to remove"))
                        .highlight_style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)).highlight_symbol("> ");
                    f.render_stateful_widget(list, area, &mut sel_state);
                }
                UiMode::Message(msg) => {
                    let w = (size.width / 3).max(20);
                    let h = 3;
                    let area = ratatui::layout::Rect::new((size.width.saturating_sub(w))/2, (size.height.saturating_sub(h))/2, w, h);
                    let p = Paragraph::new(msg.clone()).style(Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)).block(Block::default().borders(Borders::ALL).title("Info"));
                    f.render_widget(Clear, area);
                    f.render_widget(p, area);
                }
            }
        }).unwrap();

        // Handle input
        let ev = event::read().unwrap();
        match ev {
            Event::Key(key) => {
                // handle focus switching
                match key.code {
                    KeyCode::Tab => {
                        focus = match focus {
                            Focus::Actions => Focus::Aliases,
                            Focus::Aliases => Focus::Actions,
                        };
                        // ensure states have a selected item
                        if let Focus::Aliases = focus {
                            if alias_state.selected().is_none() && !aliases.is_empty() { alias_state.select(Some(0)); }
                        } else {
                            opt_state.select(Some(selected_opt));
                        }
                        continue;
                    }
                    _ => {}
                }

                match &mut ui_mode {
                    UiMode::Main => {
                        match focus {
                            Focus::Actions => {
                                match key.code {
                                    KeyCode::Up => {
                                        if selected_opt == 0 { selected_opt = options.len()-1 } else { selected_opt -= 1 }
                                        opt_state.select(Some(selected_opt));
                                    }
                                    KeyCode::Down => { selected_opt = (selected_opt+1) % options.len(); opt_state.select(Some(selected_opt)); }
                                    KeyCode::Enter => {
                                        match selected_opt {
                                            0 => { ui_mode = UiMode::Adding { step: 1, name: String::new(), command: String::new(), keybind: None }; }
                                            1 => { ui_mode = if aliases.is_empty() { UiMode::Main } else { UiMode::EditingSelect }; }
                                            2 => {
                                                if aliases.is_empty() {
                                                    ui_mode = UiMode::Message("No aliases to remove".to_string());
                                                } else {
                                                    ui_mode = UiMode::RemovingSelect;
                                                }
                                            }
                                            3 => { // Go to shell
                                                // leave TUI and spawn user's default shell
                                                disable_raw_mode().ok();
                                                execute!(terminal.backend_mut(), LeaveAlternateScreen).ok();
                                                let shell = default_shell.clone();
                                                let child = Command::new(shell).spawn();
                                                match child {
                                                    Ok(mut c) => { let _ = c.wait(); }
                                                    Err(e) => { println!("Failed to spawn shell: {}", e); }
                                                }
                                                // re-enter TUI
                                                execute!(std::io::stdout(), EnterAlternateScreen).ok();
                                                enable_raw_mode().ok();
                                                terminal = Terminal::new(CrosstermBackend::new(std::io::stdout())).unwrap();
                                            }
                                            4 => { // Quit shell
                                                disable_raw_mode().ok();
                                                terminal.clear().ok();
                                                execute!(terminal.backend_mut(), LeaveAlternateScreen).ok();
                                                return;
                                            }
                                            _ => {}
                                        }
                                    }
                                    KeyCode::Char(c) => {
                                        // trigger alias by keybind
                                        if let Some(idx) = aliases.iter().position(|a| a.keybind == Some(c)) {
                                            // Run alias
                                            let cmd = aliases[idx].command.clone();
                                            // leave alternate screen and run
                                            disable_raw_mode().ok();
                                            execute!(terminal.backend_mut(), LeaveAlternateScreen).ok();
                                            run_shell_command_with_shell(&cmd, &default_shell);
                                            // after key press, re-enter
                                            execute!(std::io::stdout(), EnterAlternateScreen).ok();
                                            enable_raw_mode().ok();
                                            terminal = Terminal::new(CrosstermBackend::new(std::io::stdout())).unwrap();
                                        }
                                    }
                                    _ => {}
                                }
                            }
                            Focus::Aliases => {
                                match key.code {
                                    KeyCode::Up => { if !aliases.is_empty() {
                                            let i = alias_state.selected().unwrap_or(0);
                                            let new = if i == 0 { aliases.len()-1 } else { i-1 };
                                            alias_state.select(Some(new));
                                        }
                                    }
                                    KeyCode::Down => { if !aliases.is_empty() {
                                            let i = alias_state.selected().unwrap_or(0);
                                            let new = (i+1) % aliases.len();
                                            alias_state.select(Some(new));
                                        }
                                    }
                                    KeyCode::Enter => {
                                        if let Some(i) = alias_state.selected() {
                                            let cmd = aliases[i].command.clone();
                                            disable_raw_mode().ok();
                                            execute!(terminal.backend_mut(), LeaveAlternateScreen).ok();
                                            run_shell_command_with_shell(&cmd, &default_shell);
                                            execute!(std::io::stdout(), EnterAlternateScreen).ok();
                                            enable_raw_mode().ok();
                                            terminal = Terminal::new(CrosstermBackend::new(std::io::stdout())).unwrap();
                                        }
                                    }
                                    _ => {}
                                }
                            }
                        }
                    }
                    UiMode::Adding { step, name, command, keybind } => {
                        match key.code {
                            KeyCode::Esc => { ui_mode = UiMode::Main; }
                            KeyCode::Enter => {
                                if *step == 1 { *step = 2; }
                                else if *step == 2 { *step = 3; }
                                else {
                                    // finalize
                                    aliases.push(Alias { name: name.clone(), command: command.clone(), keybind: *keybind });
                                    write_config(&cfg_path, &aliases, &default_shell);
                                    // update alias_state
                                    if alias_state.selected().is_none() {
                                        alias_state.select(Some(0));
                                    } else {
                                        alias_state.select(Some(aliases.len().saturating_sub(1)));
                                    }
                                    ui_mode = UiMode::Main;
                                }
                            }
                            KeyCode::Backspace => {
                                if *step == 1 { name.pop(); } else if *step == 2 { command.pop(); } else { /* keybind step - ignore */ }
                            }
                            KeyCode::Char(c) => {
                                if *step == 1 { name.push(c); }
                                else if *step == 2 { command.push(c); }
                                else if *step == 3 {
                                    *keybind = Some(c);
                                }
                            }
                            _ => {}
                        }
                    }
                    UiMode::EditingSelect => {
                        // navigate aliases and select using alias_state
                        match key.code {
                            KeyCode::Up => {
                                if aliases.is_empty() { continue };
                                let i = alias_state.selected().unwrap_or(0);
                                let new = if i == 0 { aliases.len()-1 } else { i-1 };
                                alias_state.select(Some(new));
                            }
                            KeyCode::Down => { if !aliases.is_empty() { let i = alias_state.selected().unwrap_or(0); alias_state.select(Some((i+1) % aliases.len())); } }
                            KeyCode::Enter => {
                                if let Some(idx) = alias_state.selected() {
                                    let cur_cmd = aliases[idx].command.clone();
                                    ui_mode = UiMode::Editing { index: idx, command: cur_cmd };
                                }
                            }
                            KeyCode::Esc => { ui_mode = UiMode::Main; }
                            _ => {}
                        }
                    }
                    UiMode::Editing { index, command } => {
                        match key.code {
                            KeyCode::Esc => { ui_mode = UiMode::Main; }
                            KeyCode::Enter => {
                                if let Some(a) = aliases.get_mut(*index) { a.command = command.clone(); }
                                write_config(&cfg_path, &aliases, &default_shell);
                                ui_mode = UiMode::Main;
                            }
                            KeyCode::Backspace => { command.pop(); }
                            KeyCode::Char(c) => { command.push(c); }
                            _ => {}
                        }
                    }
                    UiMode::RemovingSelect => {
                        match key.code {
                            KeyCode::Up => {
                                if aliases.is_empty() { ui_mode = UiMode::Main; continue };
                                let i = alias_state.selected().unwrap_or(0);
                                let new = if i == 0 { aliases.len()-1 } else { i-1 };
                                alias_state.select(Some(new));
                            }
                            KeyCode::Down => {
                                if !aliases.is_empty() {
                                    let i = alias_state.selected().unwrap_or(0);
                                    alias_state.select(Some((i+1) % aliases.len()));
                                }
                            }
                            KeyCode::Enter => {
                                if let Some(idx) = alias_state.selected() {
                                    aliases.remove(idx);
                                    write_config(&cfg_path, &aliases, &default_shell);
                                    // update alias_state selection
                                    if aliases.is_empty() { alias_state.select(None); } else { alias_state.select(Some(0)); }
                                    ui_mode = UiMode::Main;
                                }
                            }
                            KeyCode::Esc => { ui_mode = UiMode::Main; }
                            _ => {}
                        }
                    }
                    UiMode::Message(_) => {
                        // any key dismisses the message
                        ui_mode = UiMode::Main;
                    }
                }
            }
            Event::Resize(_, _) => { /* simply redraw on next loop */ }
            _ => {}
        }
    }
}
