# Google OAuth 2.0 & YouTube Data API v3 Setup Guide

FluxTube supports **Guest Mode** (public videos via YouTube Data API v3 Key, or offline demo fixtures when no key is set) and full **Google OAuth 2.0** sign-in on Windows 10/11 and macOS 11 Big Sur+.

## 1. Create a Google Cloud Project & Enable YouTube Data API v3
1. Open [Google Cloud Console](https://console.cloud.google.com/).
2. Create or select a project (e.g., `FluxTube-Desktop`).
3. Go to **APIs & Services -> Library**, search for **YouTube Data API v3**, and click **Enable**.

## 2. Optional: API Key for Guest / Public Browsing
1. Go to **APIs & Services -> Credentials -> Create Credentials -> API key**.
2. Restrict the key to **YouTube Data API v3**.
3. Paste it into **Settings -> Google API -> YOUTUBE_API_KEY** (or `.env` as `YOUTUBE_API_KEY=...`).

## 3. Configure OAuth Consent Screen
1. Go to **APIs & Services -> OAuth consent screen**.
2. Choose **External** (or Internal for Google Workspace).
3. Add scopes matching FluxTube's implemented features:
   - `openid`
   - `profile`
   - `email`
   - `https://www.googleapis.com/auth/youtube.force-ssl` (covers `channels.list?mine=true`, `subscriptions`, `playlists`, `playlistItems`, `videos.getRating/rate`, and `commentThreads.insert`)
4. While the app is in **Testing** status, add your Google account email under **Test users**.

## 4. Create OAuth 2.0 Client ID (Desktop App JSON — Recommended)
1. Go to **APIs & Services -> Credentials -> Create Credentials -> OAuth client ID**.
2. Select **Application type: Desktop app** (`installed` JSON).
3. Click **Download JSON** (`client_secret_*.json`).
4. In FluxTube:
   - Open **Settings -> Google API** (or click **Sign In -> Import OAuth JSON...**).
   - Select your downloaded `client_secret_*.json` file.
   - Click **Sign In** on the **Browser PKCE** tab. FluxTube opens your system browser with PKCE S256 and binds a temporary `http://127.0.0.1:<random_port>/callback` loopback listener (supported automatically by Google Desktop OAuth clients).

> **Note on Desktop Public Clients**: Desktop OAuth clients cannot keep a compiled secret private in client binaries; FluxTube enforces PKCE S256 (`code_challenge_method=S256`) and stores your OAuth refresh token strictly in **macOS Keychain** or **Windows Credential Manager**, never in plaintext SQLite/logs.

### Optional Alternative: TV / Device Code Flow (`google.com/device`)
If you prefer entering a short pairing code on `https://www.google.com/device`, create an OAuth Client ID with type **TVs and Limited Input devices** and enter its Client ID + Client Secret under **Settings -> Google API**.

## 5. Quota & Diagnostics
- Default YouTube Data API v3 quota is **10,000 units/day** (resets midnight Pacific Time).
- `search.list` costs **100 units**; `videos.list`, `channels.list`, `subscriptions.list`, `playlists.list`, and `commentThreads.list` cost **1 unit**; write actions (`commentThreads.insert`, `videos.rate`, `playlists.insert`, `subscriptions.insert/delete`) cost **50 units**.
- Open **Settings -> Google API** anytime to inspect OAuth config status, connected YouTube channel, granted scopes, last token refresh timestamp, API status/errors, quota usage, or to **Revoke Google Access** (`https://oauth2.googleapis.com/revoke`).
