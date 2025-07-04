use color_eyre::Result;
use futures_util::StreamExt;
use indicatif::{ProgressBar, ProgressStyle};
use ratatui::{
    crossterm::{
        event::{self, Event, KeyCode, KeyEventKind},
        execute,
        terminal::{disable_raw_mode, enable_raw_mode, Clear, ClearType},
    },
    prelude::*,
    widgets::{List, ListItem, ListState, Paragraph},
};
use reqwest::Client;
use serde::Deserialize;
use tokio::{
    fs::File,
    io::{self, AsyncBufReadExt, AsyncWriteExt, BufReader},
};

/// ───────────────────────── Data models ──────────────────────────
#[derive(Debug, Deserialize)]
struct ServerListResponse {
    data: Vec<ServerData>,
}
#[derive(Debug, Deserialize)]
struct ServerData {
    attributes: Server,
}
#[derive(Debug, Deserialize)]
struct Server {
    identifier: String,
    uuid: String,
    name: String,
}

#[derive(Debug, Deserialize)]
struct BackupListResponse {
    data: Vec<BackupData>,
}
#[derive(Debug, Deserialize)]
struct BackupData {
    attributes: Backup,
}
#[derive(Debug, Deserialize)]
struct Backup {
    uuid: String,
    name: String,
    created_at: String,
    bytes: u64,
}

#[derive(Debug, Deserialize)]
struct BackupDownloadLinkResponse {
    attributes: Attributes,
}
#[derive(Debug, Deserialize)]
struct Attributes {
    url: String,
}

/// ─────────────────────────── main ───────────────────────────────
#[tokio::main]
async fn main() -> Result<()> {
    if let Err(e) = run().await {
        eprintln!("\x1b[1;31m❌ Error:\x1b[0m {:?}", e);
        println!("\x1b[1;90m🔚 Press Enter to exit...\x1b[0m");
        let mut input = String::new();
        std::io::stdin().read_line(&mut input).unwrap();
    }
    Ok(())
}

/// ─────────────────────────── run ────────────────────────────────
async fn run() -> Result<()> {
    color_eyre::install()?;

    // ── Gather API key ──
    println!("\x1b[1;34m🔐 Enter your \x1b[1;36mATBP Hosting\x1b[1;34m API key:\x1b[0m");
    let mut api_key = String::new();
    let stdin = io::stdin();
    let mut reader = BufReader::new(stdin);
    reader.read_line(&mut api_key).await?;
    let api_key = api_key.trim().to_string();

    // ── Fixed panel URL ──
    let panel_url = "https://panel.atbphosting.com";

    execute!(std::io::stdout(), Clear(ClearType::All))?;

    // ── HTTP client ──
    let client = Client::builder()
        .user_agent("ClumsyLoader/0.1")
        .build()?;

    // ── Server‑selection loop ──
    'server_loop: loop {
        let servers = fetch_servers(&client, panel_url, &api_key).await?;
        if servers.is_empty() {
            println!("\x1b[1;31m⚠️  No servers were found in your account.\x1b[0m");
            break;
        }

        let server_display: Vec<String> = std::iter::once("← Back".to_string())
            .chain(servers.iter().map(|s| s.name.clone()))
            .collect();

        match select_from_list("\x1b[1;33m🎮 Select a Server\x1b[0m", &server_display).await? {
            None | Some(0) => break,
            Some(sel) => {
                let sel = sel - 1;
                let server_uuid = &servers[sel].uuid;
                let server_short = &servers[sel].identifier;

                // ── Backup‑selection loop ──
                loop {
                    let backups =
                        fetch_backups(&client, panel_url, &api_key, server_uuid).await?;
                    if backups.is_empty() {
                        println!(
                            "\x1b[1;31m📦 No backups available for the selected server.\x1b[0m"
                        );
                        continue 'server_loop;
                    }

                    let backup_display: Vec<String> = std::iter::once("← Back".to_string())
                        .chain(
                            backups
                                .iter()
                                .map(|b| format!("{} - {}", b.name, b.created_at)),
                        )
                        .collect();

                    match select_from_list(
                        "\x1b[1;33m📦 Select a Backup\x1b[0m",
                        &backup_display,
                    )
                    .await?
                    {
                        None | Some(0) => continue 'server_loop,
                        Some(bi) => {
                            let bi = bi - 1;
                            let back_url = generate_backup_dl_link(
                                &client,
                                panel_url,
                                &api_key,
                                server_short,
                                &backups[bi].uuid,
                            )
                            .await?;

                            download_backup(
                                &client,
                                &back_url,
                                &backups[bi].uuid,
                                backups[bi].bytes,
                            )
                            .await?;
                            break 'server_loop; // exit after download
                        }
                    }
                }
            }
        }
    }

    Ok(())
}

/// ───────────────────────── Networking helpers ───────────────────
async fn fetch_servers(client: &Client, url: &str, api_key: &str) -> Result<Vec<Server>> {
    let response = client
        .get(format!("{url}/api/client"))
        .header("Authorization", format!("Bearer {api_key}"))
        .header("Accept", "application/json")
        .send()
        .await?
        .json::<ServerListResponse>()
        .await?;
    Ok(response.data.into_iter().map(|s| s.attributes).collect())
}

async fn fetch_backups(
    client: &Client,
    url: &str,
    api_key: &str,
    server_uuid: &str,
) -> Result<Vec<Backup>> {
    let response = client
        .get(format!("{url}/api/client/servers/{server_uuid}/backups"))
        .header("Authorization", format!("Bearer {api_key}"))
        .header("Accept", "application/json")
        .send()
        .await?
        .json::<BackupListResponse>()
        .await?;
    Ok(response.data.into_iter().map(|b| b.attributes).collect())
}

async fn generate_backup_dl_link(
    client: &Client,
    url: &str,
    api_key: &str,
    server_uuid: &str,
    backup_uuid: &str,
) -> Result<String> {
    let response = client
        .get(format!(
            "{url}/api/client/servers/{server_uuid}/backups/{backup_uuid}/download"
        ))
        .header("Authorization", format!("Bearer {api_key}"))
        .header("Accept", "application/json")
        .send()
        .await?;
    Ok(response
        .json::<BackupDownloadLinkResponse>()
        .await?
        .attributes
        .url)
}

/// ───────────────────────── Download helper ──────────────────────
async fn download_backup(
    client: &Client,
    url: &str,
    backup_uuid: &str,
    backup_bytes: u64,
) -> Result<()> {
    let resp = client.get(url).send().await?;
    if resp
        .headers()
        .get("Content-Type")
        .map_or(false, |v| v == "text/html")
    {
        let err_msg = resp.text().await?;
        return Err(color_eyre::eyre::eyre!("Failed to download backup: {err_msg}"));
    }

    let pb = ProgressBar::new(backup_bytes);
    pb.set_style(
        ProgressStyle::with_template(
            "{spinner:.green} [{wide_bar:.cyan/blue}] {bytes}/{total_bytes} ({eta})",
        )
        .unwrap()
        .progress_chars("#>-"),
    );

    let mut file = File::create(format!("{backup_uuid}.tar.gz")).await?;
    let mut downloaded: u64 = 0;
    let mut stream = resp.bytes_stream();

    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        downloaded += chunk.len() as u64;
        pb.set_position(downloaded);
        file.write_all(&chunk).await?;
    }

    pb.finish_with_message("\x1b[1;32m✅ Download complete!\x1b[0m");
    Ok(())
}

/// ───────────────────────── UI helper ────────────────────────────
async fn select_from_list(title: &str, items: &[String]) -> Result<Option<usize>> {
    enable_raw_mode()?;
    let mut terminal = Terminal::new(CrosstermBackend::new(std::io::stdout()))?;

    let mut state = ListState::default();
    state.select(Some(0));

    loop {
        terminal.draw(|frame| {
            let area = frame.area();
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .margin(1)
                .constraints([Constraint::Length(3), Constraint::Min(1)].as_ref())
                .split(area);

            let title_paragraph = Paragraph::new(title)
                .style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
                .alignment(Alignment::Center);
            frame.render_widget(title_paragraph, chunks[0]);

            let list_items: Vec<ListItem> =
                items.iter().map(|i| ListItem::new(i.clone())).collect();

            let list = List::new(list_items)
                .highlight_symbol("> ")
                .highlight_style(
                    Style::default().fg(Color::LightBlue).add_modifier(Modifier::BOLD),
                );

            frame.render_stateful_widget(list, chunks[1], &mut state);
        })?;

        if let Event::Key(key) = event::read()? {
            if key.kind == KeyEventKind::Press {
                match key.code {
                    KeyCode::Esc | KeyCode::Char('q') => {
                        disable_raw_mode()?;
                        execute!(std::io::stdout(), Clear(ClearType::All))?;
                        return Ok(None);
                    }
                    KeyCode::Down => state.select_next(),
                    KeyCode::Up => state.select_previous(),
                    KeyCode::Enter => {
                        disable_raw_mode()?;
                        execute!(std::io::stdout(), Clear(ClearType::All))?;
                        return Ok(state.selected());
                    }
                    _ => {}
                }
            }
        }
    }
}
