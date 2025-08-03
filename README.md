# uptime

⚠️ uptime.noratrieb.dev has been retired ⚠️

custom uptime monitoring tool.

## config

JSON file located at `$UPTIME_CONFIG_PATH`, defaults to `./uptime.json`.

```json
{
  "interval_seconds": 30,
  "websites": [
    {
      "name": "nilstrieb.dev",
      "url": "https://nilstrieb.dev"
    },
    {
      "name": "google.com",
      "url": "https://google.com"
    }
  ],
  "db_url": "sqlite::memory:"
}
```

`db_url` can be overriden with `$UPTIME_DB_URL` and defaults to `./uptime.db` if not present.
