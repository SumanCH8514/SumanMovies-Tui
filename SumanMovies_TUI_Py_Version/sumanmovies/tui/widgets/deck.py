"""Landing Deck widget with Discover Categories, Continue Watching, and Favorites tabs."""

from typing import List, Callable, Optional
from textual.widget import Widget
from textual.widgets import TabbedContent, TabPane, ListView, ListItem, Label, Static
from rich.text import Text
from ...models import CatalogItem, WatchHistoryItem


class LandingDeck(Widget):
    def __init__(
        self,
        on_select_category: Optional[Callable[[str], None]] = None,
        on_select_history: Optional[Callable[[WatchHistoryItem], None]] = None,
        on_select_favorite: Optional[Callable[[CatalogItem], None]] = None,
        id: Optional[str] = "landing-deck",
    ):
        super().__init__(id=id)
        self.on_select_category = on_select_category
        self.on_select_history = on_select_history
        self.on_select_favorite = on_select_favorite

    def compose(self):
        with TabbedContent(initial="discover-tab", id="deck-tabs"):
            with TabPane("✦ Discover Categories", id="discover-tab"):
                with ListView(id="discover-list"):
                    yield ListItem(
                        Label("🔥  Trending Now (Movies & Shows)"),
                        id="cat_trending",
                        classes="deck-item-row",
                    )
                    yield ListItem(
                        Label("⭐  Top Rated & Popular Series"),
                        id="cat_series",
                        classes="deck-item-row",
                    )
                    yield ListItem(
                        Label("✨  Latest HD & 4K Releases"),
                        id="cat_latest",
                        classes="deck-item-row",
                    )
                    yield ListItem(
                        Label("🎬  4K UHD Cinema (4KHDHub)"),
                        id="cat_4k",
                        classes="deck-item-row",
                    )

            with TabPane("Continue Watching", id="continue-tab"):
                yield ListView(id="continue-list")

            with TabPane("Favorites", id="favorites-tab"):
                yield ListView(id="favorites-list")
