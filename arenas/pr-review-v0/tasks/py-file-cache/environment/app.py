import requests

import cache


def fetch_profile(user_id):
    key = f"profile-{user_id}"
    cached = cache.get(key)
    if cached is not None:
        return cached
    resp = requests.get(f"https://api.example.com/users/{user_id}")
    resp.raise_for_status()
    data = resp.json()
    cache.set(key, data)
    return data
