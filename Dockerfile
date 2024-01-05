FROM python:3.11-slim as poetry

RUN pip install poetry poetry-plugin-export

COPY poetry.lock pyproject.toml ./

RUN poetry export -f requirements.txt -o requirements.txt

FROM python:3.11-slim

WORKDIR /app

COPY --from=poetry requirements.txt requirements.txt

RUN pip install -r requirements.txt

COPY plex_transmission_throttler.py plex_transmission_throttler.py

EXPOSE 8000

CMD ["python", "plex_transmission_throttler.py"]
