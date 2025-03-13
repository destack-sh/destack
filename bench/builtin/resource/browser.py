from typing import TYPE_CHECKING, Annotated, Any, cast, override

from bench.language import (
    Browser,
    BrowserType,
    File,
    FileFormat,
    FileType,
    NodeMode,
    Page,
    upload_file,
)

from .computer import ComputerKit, IComputer

if TYPE_CHECKING:
    from playwright.async_api import Page as PlaywrightPage

    from bench.runtime.core.runner import Runner


from bench.builtin.core import class_to_kit

# ruff: noqa: N802,N803


class IBrowser(IComputer, Runner if TYPE_CHECKING else object):
    """The common interface for a Browser."""

    def _get_pw_page(self, browser: Browser) -> "PlaywrightPage":
        """Get the Playwright Page for the Browser."""
        ...  # nocheckin

    #
    # Computer
    #

    @override
    async def Screenshot(self) -> Annotated[dict[str, File], {"image": File}]:
        browser = self._get_ready_resource_or_error(Browser)
        pw_page = self._get_pw_page(browser)
        screenshot_bytes = await pw_page.screenshot(full_page=False, animations="disabled")
        now = self.session._oracle.utc()
        screenshot = await upload_file(
            screenshot_bytes,
            type=FileType.IMAGE,
            format=FileFormat.PNG,
            name=f"{browser.name} Screenshot {now.strftime('%Y-%m-%d %H:%M:%S.%f')}",
        )
        return {"image": screenshot}

    @override
    async def Click(self, X: int, Y: int, Button: str = "left") -> None:
        browser = self._get_ready_resource_or_error(Browser)
        pw_page = self._get_pw_page(browser)
        if Button == "back":
            await pw_page.go_back()
        elif Button == "forward":
            await pw_page.go_forward()
        elif Button == "wheel":
            await pw_page.mouse.wheel(X, Y)
        else:
            button_mapping = {"left": "left", "middle": "middle", "right": "right"}
            button_type = button_mapping.get(Button, "left")
            await pw_page.mouse.click(X, Y, button=cast(Any, button_type))

    @override
    async def Double_Click(self, X: int, Y: int) -> None:
        browser = self._get_ready_resource_or_error(Browser)
        pw_page = self._get_pw_page(browser)
        await pw_page.mouse.dblclick(X, Y)

    @override
    async def Press(self, Keys: list[str]) -> None:
        browser = self._get_ready_resource_or_error(Browser)
        pw_page = self._get_pw_page(browser)
        for key in Keys:
            await pw_page.keyboard.press(key)

    @override
    async def Type(self, Text: str) -> None:
        browser = self._get_ready_resource_or_error(Browser)
        pw_page = self._get_pw_page(browser)
        await pw_page.keyboard.type(Text)

    @override
    async def Move(self, X: int, Y: int) -> None:
        browser = self._get_ready_resource_or_error(Browser)
        pw_page = self._get_pw_page(browser)
        await pw_page.mouse.move(X, Y)

    @override
    async def Scroll(self, X: int, Y: int, Scroll_X: int, Scroll_Y: int) -> None:
        browser = self._get_ready_resource_or_error(Browser)
        pw_page = self._get_pw_page(browser)
        await pw_page.mouse.wheel(Scroll_X, Scroll_Y)

    #
    # Browser-specific
    #

    async def Get_Current_Url(self) -> Annotated[dict[str, Any], {"url": str}]:
        """Get the current URL of the browser."""
        browser = self._get_ready_resource_or_error(Browser)
        pw_page = self._get_pw_page(browser)
        return {"url": pw_page.url}

    async def Go_To_Url(self, Url: str) -> None:
        """Go to a URL."""
        browser = self._get_ready_resource_or_error(Browser)
        pw_page = self._get_pw_page(browser)
        await pw_page.goto(Url)

    async def Go_Forward(self) -> None:
        """Go forward in the browser history."""
        browser = self._get_ready_resource_or_error(Browser)
        pw_page = self._get_pw_page(browser)
        await pw_page.go_forward()

    async def Go_Back(self) -> None:
        """Go back in the browser history."""
        browser = self._get_ready_resource_or_error(Browser)
        pw_page = self._get_pw_page(browser)
        await pw_page.go_back()


BrowserKit = class_to_kit(IBrowser, "Browser", template=ComputerKit)
ChromeBrowser = Browser(name="Chrome Browser", type=BrowserType.CHROMIUM, mode=NodeMode.TEMPLATE)
BrowserPage = Page.new("Browser", ChromeBrowser, BrowserKit)
