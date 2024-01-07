# plex-transmission-throttler

A Rust service handling [Tautulli Webhooks](https://tautulli.com/) to enable alt speed
on [Transmission](https://transmissionbt.com/) torrent client.

## Configure it

### Required env variables

`plex-transmission-throttler` requires the following env variables:

```properties
TRANSMISSION_URL=<TRANSMISSION_HOST>/transmission/rpc
TRANSMISSION_USERNAME=transmission
TRANSMISSION_PASSWORD=<TRANSMISSION_PASSWORD>
```

You can create a `.env` file with the previous content or `export` them in your current shell.

#### Tautulli Webhook Configuration

1. Setup a new [
   _Webhook_ notification agent](https://github.com/Tautulli/Tautulli/wiki/Notification-Agents-Guide#webhook)
2. Enter the url to your running instance of `plex-transmission-throttler` with a trailing `/`.
   Ex: `http://localhost:8000/`
3. Set _Webhook Method_ to `POST`
4. In _Triggers_, enable :
    * Playback Start
    * Playback Stop
    * Playback Pause
    * Playback Resume
    * Buffer Warning
5. In _Data_, set _JSON Data_ for each of the previous triggers to:
    ```json
    {
        "action": "play",
        "stream_location": "{stream_location}"
    }
    ```
   The `action` field should be set as follows:
   | Trigger | action |
   |-----------------|--------|
   | Playback Start | play |
   | Playback Stop | stop |
   | Playback Pause | pause |
   | Playback Resume | resume |
   | Buffer Warning | play |
6. Save trigger

## Run it

### Docker

```shell
docker run -p 8000:8000 --env-file .env ghcr.io/technophil98/plex-transmission-throttler:latest
```

### Locally

```shell
# Export variables in .env to current shell
set -o allexport; source .env; set +o allexport
# Run it! Will be accessible at 'localhost:8000'
cargo run
```
