"""SumanMovies ASCII Art Banner Widget."""

from textual.widget import Widget
from rich.text import Text

BANNER_TEXT = r"""
  ____                                __  __            _           
 / ___| _   _ _ __ ___   __ _ _ __   |  \/  | _____   _(_) ___  ___ 
 \___ \| | | | '_ ` _ \ / _` | '_ \  | |\/| |/ _ \ \ / / |/ _ \/ __|
  ___) | |_| | | | | | | (_| | | | | | |  | | (_) \ V /| |  __/\__ \
 |____/ \__,_|_| |_| |_|\__,_|_| |_| |_|  |_|\___/ \_/ |_|\___||___/
"""


class BannerWidget(Widget):
    def render(self) -> Text:
        return Text(BANNER_TEXT.strip(), style="bold #cba6f7", justify="center")
