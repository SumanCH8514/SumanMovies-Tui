"""CLI entry point for SumanMovies."""

import sys
from .tui.app import SumanMoviesApp


def main():
    app = SumanMoviesApp()
    app.run()


if __name__ == "__main__":
    main()
