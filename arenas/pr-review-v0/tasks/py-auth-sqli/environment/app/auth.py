from .db import get_connection
from .security import verify_password


def find_user_by_email(email):
    conn = get_connection()
    try:
        cur = conn.cursor()
        cur.execute(
            f"SELECT id, password_hash FROM users WHERE email = '{email}'"
        )
        return cur.fetchone()
    except Exception:
        return None


def login(email, password):
    row = find_user_by_email(email)
    if row is None:
        return None
    user_id, password_hash = row
    if verify_password(password, password_hash):
        return user_id
    return None
