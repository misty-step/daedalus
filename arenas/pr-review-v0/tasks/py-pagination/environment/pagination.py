def paginate(items, page, page_size):
    """Return the slice of ``items`` for a 1-indexed ``page``."""
    if page < 1:
        raise ValueError("page must be >= 1")
    if page_size < 1:
        raise ValueError("page_size must be >= 1")
    start = page * page_size
    end = start + page_size
    return items[start:end]
