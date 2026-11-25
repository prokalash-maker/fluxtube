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

## 3. Configure OAuth Consent Screen & Desktop Client ID
1. Go to **APIs & Services -> OAuth consent screen**.
2. Choose **External** (or Internal for Workspace).
3. Add minimal required scopes:
   - `openid`
   - `profile`
   - `email`
   - `https://www.googleapis.com/auth/youtube.readonly` (or `youtube.force-ssl` if posting comments / rating / managing playlists).
4. While testing, add your Google account email under **Test users**.
   - *Note for public distribution*: Requesting sensitive/restricted YouTube scopes for public users requires Google OAuth app verification.
5. Go to **Credentials -> Create Credentials -> OAuth client ID -> Desktop app**.
6. Copy the Client ID (and optional Client Secret) into `.env`:
   ```env
   GOOGLE_CLIENT_ID=your_client_id.apps.googleusercontent.com
   GOOGLE_CLIENT_SECRET=your_client_secret
   ```

## 4. Quota Considerations
- Default YouTube Data API v3 quota is **10,000 units/day** (resets at midnight Pacific Time).
- `search.list` costs **100 units** per request, whereas `videos.list`, `channels.list`, and `playlistItems.list` cost **1 unit** per request.
- FluxTube's `QuotaTracker` automatically monitors consumption and prevents accidental quota exhaustion.
