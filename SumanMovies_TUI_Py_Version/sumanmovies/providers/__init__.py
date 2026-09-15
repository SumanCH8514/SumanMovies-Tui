"""Streaming providers package for SumanMovies TUI."""

from .base import BaseProvider
from .moviebox import MovieBoxProvider
from .fourkhdhub import FourKHDHubProvider

PROVIDERS = {
    "moviebox": MovieBoxProvider,
    "4khdhub": FourKHDHubProvider,
}
