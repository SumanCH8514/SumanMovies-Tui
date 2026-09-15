"""Settings and maintenance screen."""

from textual.screen import Screen
from textual.containers import Vertical, Horizontal
from textual.widgets import Label, Button, ListView, ListItem, Footer, Static
from ...cache import clear_all_cache
from ...player import detect_players


class SettingsScreen(Screen):
    BINDINGS = [
        ("escape", "go_back", "Back"),
    ]

    def compose(self):
        detected = ", ".join(detect_players()) or "None detected (Install MPV or VLC)"
        with Vertical(id="details-container"):
            yield Label("⚙️  Settings & Preferences", id="details-title")
            with ListView(id="settings-list"):
                yield ListItem(
                    Label(f"Default Media Player: [Auto] (Detected: {detected})"),
                    id="set_player",
                    classes="deck-item-row",
                )
                yield ListItem(
                    Label("Theme: [Catppuccin Mocha]"),
                    id="set_theme",
                    classes="deck-item-row",
                )
                yield ListItem(
                    Label("🗑️  Clear Disk Cache (Remove temporary images & responses)"),
                    id="set_cache",
                    classes="deck-item-row",
                )
                yield ListItem(
                    Label("🌐  GitHub Repository (https://github.com/SumanCH8514/SumanMovies-Tui)"),
                    id="set_github",
                    classes="deck-item-row",
                )
            yield Button("◀ Back (Esc)", id="btn-back", variant="primary")
            yield Footer()

    def on_list_view_selected(self, event: ListView.Selected):
        item_id = event.item.id
        if item_id == "set_cache":
            count = clear_all_cache()
            self.app.notify(f"Disk cache cleared! ({count} files removed)")
        elif item_id == "set_github":
            import webbrowser
            webbrowser.open("https://github.com/SumanCH8514/SumanMovies-Tui")
            self.app.notify("Opening GitHub repository...")

    def on_button_pressed(self, event: Button.Pressed):
        if event.button.id == "btn-back":
            self.action_go_back()

    def action_go_back(self):
        self.app.pop_screen()
