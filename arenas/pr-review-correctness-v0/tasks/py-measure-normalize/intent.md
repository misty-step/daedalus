Performance: Measurement.get() is on the hot path for layout; avoid constructing three intermediate Measurement tuples per call.
