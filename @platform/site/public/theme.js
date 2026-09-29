// restore the selected theme before the page styles load
try {
    const theme = localStorage.getItem("destack-theme");
    if (theme === "light" || theme === "dark") {
        document.documentElement.dataset.theme = theme;
        document.documentElement.dataset.destackTheme = theme;
        document.documentElement.style.colorScheme = theme;
    }
} catch (error) {
    console.warn("Could not restore the site theme", error);
}
