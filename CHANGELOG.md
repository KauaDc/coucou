# Changelog

## Unreleased

- Compact island on screens without a notch (#22) — thanks @Kamasoutra
- Only web links (http/https) open from the notch; other kinds of links from Claude or integrations are ignored (#16) — thanks @Cris1670
- Hook socket limited to your own user account, with size and time limits; logs no longer keep commands, n8n data or full URLs, and stay under 1 MB (#16) — thanks @Cris1670 and @Vignesh-Thangamariappan
- The island always reopens after folding, and Settings opens below it, resizable — thanks @rouderz
- Windows: the interface is in Brazilian Portuguese when Windows is set to Portuguese, English otherwise
- Windows: with several displays, the island can be pinned to any of them, and in "Display under the cursor" mode the top of every display wakes it
- Windows: the chat can use Gemini instead of Claude — pick the provider, key and model in Settings → Chat
- Windows: Claude Code's questions (AskUserQuestion) are answered from the island — single choice in one click, multiple choice, up to 4 questions in a row, or your own words — instead of showing up as Allow / Deny
- Windows: permission requests and questions stay on the island until you answer (no more 2-minute timeout), and disappear by themselves once you answer in the terminal; Settings asks to reinstall hooks written by older versions
- Windows: Discloud integration — app status, CPU and RAM, outage alerts, recent logs, and Start / Stop / Restart buttons (Stop and Restart ask for a second click)
