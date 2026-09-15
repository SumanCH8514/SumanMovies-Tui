"""Details screen for stream selection, season/episode chooser, and player playback."""

from typing import Optional, List
from textual.screen import Screen
from textual.containers import Vertical, Horizontal, ScrollableContainer
from textual.widgets import Label, Button, ListView, ListItem, Footer, Static
from ...models import CatalogItem, MediaDetails, StreamSource, SubtitleTrack
from ...player import launch_player


class StreamItem(ListItem):
    def __init__(self, stream: StreamSource):
        super().__init__()
        self.stream = stream

    def compose(self):
        yield Label(f"▶ [{self.stream.quality}] {self.stream.source_label} ({self.stream.url[:45]}...)")


class DetailsScreen(Screen):
    BINDINGS = [
        ("escape", "go_back", "Back"),
        ("f", "toggle_favorite", "Favorite"),
        ("d", "download", "Download"),
    ]

    def __init__(self, item: CatalogItem):
        super().__init__()
        self.item = item
        self.details: Optional[MediaDetails] = None
        self.streams: List[StreamSource] = []
        self.selected_season: int = 1
        self.selected_episode: int = 1

    def compose(self):
        with Vertical(id="details-container"):
            yield Label(f"{self.item.title}", id="details-title")
            yield Label("Loading details and streaming mirrors...", id="details-overview")
            with Horizontal():
                yield Button("⭐ Toggle Favorite (f)", id="btn-fav", variant="primary")
                yield Button("◀ Back (Esc)", id="btn-back", variant="default")
            yield Label("Available Streams / Quality Mirrors:", id="streams-header")
            with ScrollableContainer():
                yield ListView(id="streams-list")
            yield Footer()

    async def on_mount(self):
        provider = self.app.providers.get(self.item.provider)
        if not provider:
            self.query_one("#details-overview", Label).update("Provider unavailable.")
            return

        self.details = await provider.get_details(self.item.id)
        if self.details:
            overview_text = self.details.overview or "No synopsis available."
            rating_text = f"⭐ {self.details.rating:.1f}" if self.details.rating else ""
            genres_text = ", ".join(self.details.genres) if self.details.genres else ""
            self.query_one("#details-overview", Label).update(
                f"{rating_text} | {genres_text}\n\n{overview_text}"
            )

        # Fetch streams
        self.streams = await provider.get_streams(
            self.item.id,
            season=self.selected_season,
            episode=self.selected_episode,
        )
        streams_list = self.query_one("#streams-list", ListView)
        await streams_list.clear()
        if not self.streams:
            await streams_list.append(ListItem(Label("No playable streams found for this release.")))
        else:
            for s in self.streams:
                await streams_list.append(StreamItem(s))

    async def on_list_view_selected(self, event: ListView.Selected):
        if isinstance(event.item, StreamItem):
            stream = event.item.stream
            self.app.notify(f"Launching media player for '{self.item.title}'...")
            
            # Record start in history
            self.app.history_manager.record_progress(
                provider=self.item.provider,
                subject_id=self.item.id,
                title=self.item.title,
                season=self.selected_season,
                episode=self.selected_episode,
                progress_sec=0,
                duration_sec=0,
                poster_url=self.item.poster_url,
            )

            # Launch MPV/VLC with custom title branding
            player_pref = self.app.settings.get("player", "auto")
            try:
                proc = await launch_player(
                    url=stream.url,
                    player_preference=player_pref,
                    title=self.item.title,
                    headers=stream.headers,
                )
                await proc.wait()
            except Exception as e:
                self.app.notify(f"Player error: {e}", severity="error")

    def on_button_pressed(self, event: Button.Pressed):
        if event.button.id == "btn-back":
            self.action_go_back()
        elif event.button.id == "btn-fav":
            self.action_toggle_favorite()

    def action_go_back(self):
        self.app.pop_screen()

    def action_toggle_favorite(self):
        is_fav = self.app.history_manager.toggle_favorite(self.item)
        status = "Added to Favorites ★" if is_fav else "Removed from Favorites"
        self.app.notify(status)
