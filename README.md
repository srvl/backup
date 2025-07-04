# ATBP Hosting bpdl

**bpdl** is a terminal-based Rust application for downloading backups from your ATBP Hosting Panel using the API.

It features a simple interactive TUI (terminal user interface) to:
- Select a server from your account.
- Choose from the list of available backups.
- Download the selected backup to your local machine.

## Why use bpdl?

ATBP Hosting's scheduled backups automatically delete the oldest backup once the limit is reached. bpdl gives you the ability to automatically grab and store that backup elsewhere before it's deleted — useful for external backups and archival.

## How to Use

**Run the binary** which can be obtained from the [releases](https://github.com/srvl/backup/releases) tab


When prompted:
Paste your ATBP Hosting Panel API key.

Enter your Panel URL (or press Enter to use the default panel.atbphosting.com).

Use the arrow keys to navigate the server and backup selection lists, then press Enter to confirm.

Your selected backup will be downloaded in your working directory.
