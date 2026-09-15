"""Home and search screen."""

from typing import List, Optional
from textual.screen import Screen
from textual.containers import Vertical, Horizontal, ScrollableContainer
from textual.widgets import Input, Static, Label, ListView, ListItem, Footer
from rich.text import Text
from ..widgets.banner import BannerWidget
from ..widgets.deck import LandingDeck
from ...models import CatalogItem, WatchHistoryItem


class MovieCard(ListItem):
    def __init__(self, item: CatalogItem):
        super().__init__(classes="movie-card")
        self.item = item

    def compose(self):
        year_str = f" ({self.item.year})" if self.item.year else ""
        type_badge = f"[{self.item.media_type.value.upper()}]"
        rating_str = f" ⭐ {self.item.rating:.1f}" if self.item.rating else ""
        
        yield Label(f"{self.item.title}{year_str}", classes="movie-card-title")
        yield Label(f"{type_badge}{rating_str} • {self.item.provider.upper()}", classes="movie-card-meta")


class HomeScreen(Screen):
    BINDINGS = [
        ("slash", "focus_search", "Search"),
        ("ctrl+s", "open_settings", "Settings"),
        ("ctrl+p", "switch_provider", "Switch Provider"),
        ("escape", "clear_or_home", "Clear"),
    ]

    def compose(self):
        with Vertical():
            yield BannerWidget(id="banner")
            with Horizontal(id="search-container"):
                yield Input(
                    placeholder="🔍 Search movies, TV series, 4K releases... (Press / to search)",
                    id="search-input",
                )
            yield LandingDeck(
                on_select_category=self.on_category_selected,
                on_select_history=self.on_history_selected,
                on_select_favorite=self.on_favorite_selected,
            )
            with ScrollableContainer(id="results-container"):
                yield ListView(id="results-list")
            yield Footer()

    async def on_mount(self):
        await self.load_continue_watching()
        await self.load_favorites()
        # Default load trending into results
        await self.load_category("cat_trending")

    async def load_continue_watching(self):
        history = self.app.history_manager.get_continue_watching()
        continue_list = self.query_one("#continue-list", ListView)
        await continue_list.clear()
        for item in history:
            pct = int((item.progress_seconds / max(item.duration_seconds, 1)) * 100)
            await continue_list.append(
                ListItem(
                    Label(f"▶ {item.title} (S{item.season}E{item.episode}) • {pct}% watched"),
                    classes="deck-item-row",
                )
            )

    async def load_favorites(self):
        favs = self.app.history_manager.get_favorites()
        fav_list = self.query_one("#favorites-list", ListView)
        await fav_list.clear()
        for item in favs:
            await fav_list.append(
                ListItem(
                    Label(f"★ {item.title} ({item.year or 'N/A'}) • {item.provider.upper()}"),
                    classes="deck-item-row",
                )
            )

    async def on_input_submitted(self, event: Input.Submitted):
        query = event.value.strip()
        if not query:
            return
        self.app.notify(f"Searching for '{query}'...")
        provider = self.app.get_active_provider()
        results = await provider.search(query)
        await self.display_results(results)

    async def display_results(self, items: List[CatalogItem]):
        results_list = self.query_one("#results-list", ListView)
        await results_list.clear()
        if not items:
            await results_list.append(ListItem(Label("No results found. Try another query or provider.")))
            return
        for item in items:
            await results_list.append(MovieCard(item))

    async def on_list_view_selected(self, event: ListView.Selected):
        item_id = event.item.id
        if item_id and item_id.startswith("cat_"):
            await self.load_category(item_id)
        elif isinstance(event.item, MovieCard):
            self.app.open_details(event.item.item)

    async def load_category(self, cat_id: str):
        provider = self.app.get_active_provider()
        self.app.notify("Loading category...")
        if cat_id == "cat_trending":
            items = await provider.get_popular()
        elif cat_id == "cat_series":
            items = await provider.get_trending_series()
        elif cat_id == "cat_latest":
            items = await provider.get_latest()
        elif cat_id == "cat_4k":
            fourk_provider = self.app.providers.get("4khdhub")
            items = await fourk_provider.get_popular() if fourk_provider else []
        else:
            items = []
        await self.display_results(items)

    def on_category_selected(self, cat_id: str):
        pass

    def on_history_selected(self, item: WatchHistoryItem):
        pass

    def on_favorite_selected(self, item: CatalogItem):
        self.app.open_details(item)

    def action_focus_search(self):
        self.query_one("#search-input", Input).focus()

    def action_open_settings(self):
        self.app.push_screen("settings")

    def action_switch_provider(self):
        self.app.cycle_provider()

    def action_clear_or_home(self):
        inp = self.query_one("#search-input", Input)
        if inp.value:
            inp.value = ""
        inp.blur()
