import sqlite3


def connect(path="billing.db"):
    conn = sqlite3.connect(path)
    conn.row_factory = sqlite3.Row
    return conn
