Reduce lock contention in Live.update(): a single attribute assignment is atomic under the GIL, so only the display mutation needs the lock.
