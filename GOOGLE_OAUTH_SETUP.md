# Google OAuth 2.0 & YouTube Data API v3 Setup Guide

FluxTube works out-of-the-box in **Offline Demo Mode** using local mock fixtures when no API key is set. To connect live YouTube data and your Google account, create your own Google Cloud Project:

## 1. Create a Google Cloud Project
1. Go to [Google Cloud Console](https://console.cloud.google.com/).
2. Create a new project (e.g., `FluxTube-Desktop`).
3. Navigate to **APIs & Services -> Library**, search for **YouTube Data API v3**, and click **Enable**.

## 2. Create a YouTube API Key (Public Data)
1. Go to **APIs & Services -> Credentials -> Create Credentials -> API key**.
2. Restrict the key to **YouTube Data API v3**.
3. Copy the key into `.env`:
   ```env
   YOUTUBE_API_KEY=your_api_key_here
   ```

## 3. Choose Your Google OAuth 2.0 Client Type

FluxTube supports **two official Google OAuth 2.0 flows**, configurable directly inside the app (**Account -> API & OAuth Credentials**) or via `.env`:

### Option A (Recommended): TV / Device Code Login (`google.com/device`)
Best for easy sign-in by entering a short one-time code at `https://www.google.com/device` (no local loopback redirect server required):
1. Go to **APIs & Services -> OAuth consent screen**.
2. Choose **External** (or Internal for Workspace) and add scopes:
   - `openid`, `profile`, `email`
   - `https://www.googleapis.com/auth/youtube.readonly`
3. While in Testing mode, add your Google account email under **Test users**.
4. Go to **Credentials -> Create Credentials -> OAuth client ID**.
5. Choose Application type: **TVs and Limited Input devices**.
6. Copy both **Client ID** and **Client Secret** (Google requires Client Secret for TV/Limited Input token polling).

### Option B: Desktop App + Browser PKCE
1. In **Credentials -> Create Credentials -> OAuth client ID**, choose **Desktop app**.
2. Copy the **Client ID** (and optional Client Secret).

You can paste `Client ID`, `Client Secret`, and `YouTube API Key` directly inside FluxTube (**Sign In -> API & OAuth Credentials**, stored in SQLite settings + OS Keychain/Credential Manager) or in `.env`:
```env
GOOGLE_CLIENT_ID=your_client_id.apps.googleusercontent.com
GOOGLE_CLIENT_SECRET=your_client_secret
YOUTUBE_API_KEY=your_api_key_here
```

> *Note for public distribution*: Requesting YouTube scopes for external users outside your Test users list requires Google OAuth app verification.

## 4. Quota Considerations
- Default YouTube Data API v3 quota is **10,000 units/day** (resets at midnight Pacific Time).
- `search.list` costs **100 units** per request, whereas `videos.list`, `channels.list`, and `playlistItems.list` cost **1 unit** per request.
- FluxTube's `QuotaTracker` automatically monitors consumption and prevents accidental quota exhaustion.
