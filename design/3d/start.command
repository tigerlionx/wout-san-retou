#!/bin/bash
# Opens the Wout San Retou 3D viewer in your browser (needs python3, which macOS has).
cd "$(dirname "$0")"
PORT=8765
(sleep 1; open "http://localhost:$PORT/") &
echo "Serving http://localhost:$PORT/  (Ctrl+C to stop)"
python3 -m http.server $PORT
