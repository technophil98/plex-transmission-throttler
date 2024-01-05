from dataclasses import dataclass

from transmission_rpc import Client
from sanic import Sanic, json
from sanic.request import Request
from sanic.log import logger
from sanic.exceptions import BadRequest
from sanic_ext import validate

SUPPORTED_ACTIONS = ["play", "pause", "stop", "resume"]
UNTHROTTLED_STREAM_LOCATION = ["lan"]

app = Sanic("PlexTransmissionThrottler")


def enable_transmission_alt_speed(c: Client):
    logger.info("Enabling alt speed")
    c.set_session(alt_speed_enabled=True)


def disable_transmission_alt_speed(c: Client):
    logger.info("Disabling alt speed")
    c.set_session(alt_speed_enabled=False)


@app.before_server_start
async def create_transmission_client(app, loop):
    app.ctx.transmission_client = Client(host=app.config.TRANSMISSION_HOST,
                                         port=int(app.config.TRANSMISSION_PORT),
                                         username=app.config.TRANSMISSION_USERNAME,
                                         password=app.config.TRANSMISSION_PASSWORD)


@dataclass
class WebhookPayload:
    action: str
    stream_location: str


@app.get("/")
async def healthcheck(request: Request):
    return json({"status": "ok"})


@app.post("/")
@validate(json=WebhookPayload)
async def webhook(request: Request, body: WebhookPayload):
    action = body.action
    stream_location = body.stream_location

    if action not in SUPPORTED_ACTIONS:
        raise BadRequest(f"Unsupported action '{action}'.")

    if stream_location in UNTHROTTLED_STREAM_LOCATION:
        message = f"Stream location '{stream_location}' does not require throttling."
        logger.info(message)
        return json({"status": message})

    logger.info(f"Received action '{action}' for stream location '{stream_location}'")

    if action in ["play", "resume"]:
        enable_transmission_alt_speed(app.ctx.transmission_client)
        return json({"status": "Throttling enabled"})
    
    elif action in ["stop", "pause"]:
        disable_transmission_alt_speed(app.ctx.transmission_client)
        return json({"status": "Throttling disabled"})


if __name__ == "__main__":
    app.run(host='0.0.0.0', port=8000)
