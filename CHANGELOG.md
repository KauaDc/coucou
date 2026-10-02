# Changelog

## Unreleased

- Compact island on screens without a notch (#22) — thanks @Kamasoutra
- Only web links (http/https) open from the notch; other kinds of links from Claude or integrations are ignored (#16) — thanks @Cris1670
- Hook socket limited to your own user account, with size and time limits; logs no longer keep commands, n8n data or full URLs, and stay under 1 MB (#16) — thanks @Cris1670 and @Vignesh-Thangamariappan
- The island always reopens after folding, and Settings opens below it, resizable — thanks @rouderz
- Windows: the interface is in Brazilian Portuguese when Windows is set to Portuguese, English otherwise
- Windows: with several displays, the island can be pinned to any of them, and in "Display under the cursor" mode the top of every display wakes it
- Windows: the chat can use Gemini instead of Claude — pick the provider, key and model in Settings → Chat
- Windows: Discloud integration — app status, CPU and RAM, outage alerts, recent logs, and Start / Stop / Restart buttons (Stop and Restart ask for a second click)
