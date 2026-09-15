"""Main Textual application class for SumanMovies."""

from pathlib import Path
from typing import Dict
from textual.app import App
from .screens.home import HomeScreen
from .screens.details import DetailsScreen
from .screens.settings import SettingsScreen
from ..config import Settings
from ..history import HistoryManager
from ..models import CatalogItem
from ..providers import PROVIDERS, BaseProvider

TCSS_PATH = Path(__file__).parent / "styles.tcss"


class SumanMoviesApp(App):
    CSS_PATH = TCSS_PATH
    TITLE = "SumanMovies TUI (Python Edition)"
    SCREENS = {
        "home": HomeScreen,
        "settings": SettingsScreen,
    }

    def __init__(self):
        super().__init__()
        self.settings = Settings()
        self.history_manager = HistoryManager()
        self.providers: Dict[str, BaseProvider] = {
            name: cls() for name, cls in PROVIDERS.items()
        }
        self.active_provider_key: str = self.settings.get("default_provider", "moviebox")

    def on_mount(self):
        self.push_screen("home")

    def get_active_provider(self) -> BaseProvider:
        return self.providers.get(self.active_provider_key) or self.providers["moviebox"]

    def cycle_provider(self):
        keys = list(self.providers.keys())
        idx = (keys.index(self.active_provider_key) + 1) % len(keys)
        self.active_provider_key = keys[idx]
        self.settings.set("default_provider", self.active_provider_key)
        provider = self.get_active_provider()
        self.notify(f"Switched provider to: {provider.display_name}")

    def open_details(self, item: CatalogItem):
        self.push_screen(DetailsScreen(item))
