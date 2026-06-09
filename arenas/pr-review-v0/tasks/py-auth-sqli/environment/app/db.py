import sqlite3

DB_PATH = "app.db"


def get_connection():
    return sqlite3.connect(DB_PATH)
