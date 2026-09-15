#!/usr/bin/env python3
"""Run SumanMovies TUI Python Edition directly."""

import sys
from pathlib import Path

# Add current directory to path
sys.path.insert(0, str(Path(__file__).parent))

from sumanmovies.cli import main

if __name__ == "__main__":
    main()
