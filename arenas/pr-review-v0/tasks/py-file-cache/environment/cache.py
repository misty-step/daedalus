import json
import os

CACHE_DIR = os.path.expanduser("~/.app-cache")


def _path(key):
    return os.path.join(CACHE_DIR, key + ".json")


def get(key):
    path = _path(key)
    if not os.path.exists(path):
        return None
    f = open(path)
    data = json.load(f)
    f.close()
    return data


def set(key, value):
    os.makedirs(CACHE_DIR, exist_ok=True)
    tmp = _path(key) + ".tmp"
    with open(tmp, "w") as f:
        json.dump(value, f)
    os.rename(tmp, _path(key))
