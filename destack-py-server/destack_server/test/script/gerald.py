# ruff: noqa
# type: ignore

import pytest
from typing import TYPE_CHECKING

if not TYPE_CHECKING:
    pytest.skip(allow_module_level=True)

# ===============================================
# Gerald/DataAnalyzer
# ===============================================
"""
Zertifikate Data Analyzer V12 - Modul 2
Analysiert Rohdaten aus Modul 1 und erstellt analysierte Excel-Dateien für den Report
"""

import logging
import re
from datetime import datetime
from pathlib import Path
from destack import *  # noqa: F403

from .scaffold import *  # noqa: F403

try:
    import pandas as pd
except ImportError:
    pd = Any


def clean_number_enhanced(value):
    try:
        return float(str(value).replace(".", "").replace(",", ".").replace("%", "").strip())
    except Exception:
        return None


def normalize_column_name(col: str) -> str:
    col = col.strip().lower()
    col = col.replace("ä", "ae").replace("ö", "oe").replace("ü", "ue").replace("ß", "ss")
    col = re.sub(r"[^a-z0-9]+", "_", col)
    col = re.sub(r"_+", "_", col)
    return col.strip("_")


@service
class DataAnalyzer(Service):
    pass

    @action
    def process_raw_data(self, df, logger):
        logger.info("🔄 Verarbeite Rohdaten...")

        df.columns = [normalize_column_name(col) for col in df.columns]
        processed_df = df.copy()

        log("📋 Spalten im DataFrame:", processed_df.columns.tolist())

        if (
            "abstand_barriere" in processed_df.columns
            and "barriere_abstand" not in processed_df.columns
        ):
            processed_df["barriere_abstand"] = processed_df["abstand_barriere"]

        try:
            for col in ["bonus_rendite_p_a", "barriere_abstand", "aufgeld"]:
                if col in processed_df.columns:
                    processed_df[col] = processed_df[col].apply(clean_number_enhanced)

            # Berechne Restlaufzeit in Tagen
            if "bewertungstag" in processed_df.columns:
                processed_df["bewertungstag_parsed"] = pd.to_datetime(
                    processed_df["bewertungstag"], dayfirst=True, errors="coerce"
                )
                heute = datetime.utcnow()
                processed_df["restlaufzeit_tage"] = (
                    processed_df["bewertungstag_parsed"] - heute
                ).dt.days

            # Standard-Kriterien
            processed_df["meets_criteria"] = (
                (
                    processed_df.get(
                        "bonus_rendite_p_a", pd.Series([0] * len(processed_df))
                    ).fillna(0)
                    > 7.0
                )
                & (
                    processed_df.get("barriere_abstand", pd.Series([0] * len(processed_df))).fillna(
                        0
                    )
                    > 25.0
                )
                & (
                    processed_df.get("aufgeld", pd.Series([99] * len(processed_df))).fillna(99)
                    <= 5.0
                )
            )

            # Bluechip Check
            bluechip_keywords = [
                "DAX",
                "EURO STOXX",
                "EUROSTOXX",
                "S&P",
                "Dow",
                "NASDAQ",
                "Allianz",
                "SAP",
                "BASF",
                "Bayer",
                "Siemens",
                "Rheinmetall",
                "Infineon",
                "Adidas",
                "BMW",
                "Mercedes",
                "Münchener Rück",
                "Volkswagen",
                "Henkel",
                "Deutsche Bank",
                "Deutsche Telekom",
                "Apple",
                "Microsoft",
                "Nvidia",
                "Amazon",
                "Meta",
                "Alphabet",
                "Visa",
                "JP Morgan",
                "Nestlé",
                "LVMH",
            ]
            processed_df["is_bluechip"] = processed_df["basiswert"].str.contains(
                "|".join(bluechip_keywords), case=False, na=False
            )

            # Strategie-Treffer (nach Projektdefinition)
            bonus = processed_df.get(
                "bonus_rendite_p_a", pd.Series([0] * len(processed_df))
            ).fillna(0)
            puffer = processed_df.get(
                "barriere_abstand", pd.Series([0] * len(processed_df))
            ).fillna(0)
            aufgeld = processed_df.get("aufgeld", pd.Series([99] * len(processed_df))).fillna(99)
            laufzeit = processed_df.get(
                "restlaufzeit_tage", pd.Series([999] * len(processed_df))
            ).fillna(999)
            basiswert = processed_df.get("basiswert", pd.Series([""] * len(processed_df))).fillna(
                ""
            )

            processed_df["meets_strategy_criteria"] = (
                (bonus >= 7.0)
                & (puffer >= 15.0)
                & (aufgeld <= 5.0)
                & (laufzeit <= 270)
                & (laufzeit >= 30)  # extreme Kurzläufer ausschließen
                & (processed_df["is_bluechip"] == True)
            )

            # Watchlist-Kandidat-Spalte nach meets_strategy_criteria – gelockerte Bluechip-Bedingung
            # processed_df['watchlist_candidate'] = (
            #     (bonus >= 7.0) &
            #     (puffer >= 15.0) &
            #     (aufgeld > 5.0) & (aufgeld <= 7.0) &
            #     (laufzeit <= 270) &
            #     (laufzeit >= 30) &
            #     (processed_df["is_bluechip"] == True)
            # )
            processed_df["watchlist_candidate"] = (
                (bonus >= 7.0)
                & (puffer >= 15.0)
                & (aufgeld > 5.0)
                & (aufgeld <= 7.0)
                & (laufzeit <= 270)
                & (laufzeit >= 30)
                # Kein Bluechip-Filter hier, um mehr Flexibilität zu prüfen
            )

            # Score
            processed_df["attractiveness_score"] = bonus * 1.5 + puffer * 0.5 - aufgeld

            if "data_source" not in processed_df.columns:
                processed_df["data_source"] = "UNKNOWN"

        except Exception as e:
            logger.error(f"❌ Fehler beim Analysieren: {e}")
            raise

        return processed_df

    def find_latest_raw_file(self, base_folder: Path) -> Path:
        candidates = list(base_folder.glob("raw_certificates_*.xlsx"))
        if not candidates:
            raise ValueError("No raw certificates found")
        return max(candidates, key=lambda f: f.stat().st_mtime)

    def analyze_excel_file(
        self, input_file: Path, output_folder: Path, logger: logging.Logger
    ) -> Path:
        logger.info(f"📖 Lade Datei: {input_file.name}")
        df = pd.read_excel(input_file, sheet_name=0)
        logger.info(f"📊 {len(df)} Zeilen geladen")

        processed_df = self.process_raw_data(df, logger)

        timestamp = datetime.now().strftime("%Y-%m-%d_%H-%M-%S")
        output_file = output_folder / f"analyzed_certificates_{timestamp}.xlsx"

        summary = {
            "total_certificates": len(processed_df),
            "meets_criteria_count": processed_df["meets_criteria"].sum(),
            "strategy_hits": processed_df["meets_strategy_criteria"].sum(),
            "bluechip_count": processed_df["is_bluechip"].sum(),
            "avg_bonus_yield_pa": processed_df.get(
                "bonus_rendite_p_a", pd.Series(dtype=float)
            ).mean(),
            "data_source": processed_df["data_source"].iloc[0],
        }

        criteria = {
            "min_bonus_yield_pa": 7.0,
            "min_barrier_distance": 15.0,
            "max_premium": 5.0,
            "restlaufzeit_max_tage": 270,
            "require_bluechip": True,
        }

        with pd.ExcelWriter(output_file) as writer:
            processed_df.to_excel(writer, sheet_name="Analyzed_Data", index=False)
            pd.DataFrame([summary]).to_excel(writer, sheet_name="Analysis_Summary", index=False)
            pd.DataFrame([criteria]).to_excel(writer, sheet_name="Investment_Criteria", index=False)

        logger.info(f"✅ Analyse gespeichert: {output_file}")
        print(f"✅ Analyse abgeschlossen: {output_file}")
        return output_file


# ===============================================
# Gerald/DataFetcher
# ===============================================
# !/usr/bin/env python3
"""
Zertifikate Data Fetcher V12 - Modul 1
Sammelt Zertifikate-Daten von OnVista (Selenium)
Output: raw_certificates.xlsx
"""

import datetime
import os
from pathlib import Path
from typing import List, Optional, Tuple

import requests
from bs4 import BeautifulSoup
from requests.adapters import HTTPAdapter
from urllib3.util.retry import Retry


@service
class DataFetcher(Service):
    user_agent: str = script.field("user_agent", 1, str)

    def create_optimized_session(self) -> requests.Session:
        """Erstellt optimierte HTTP-Session"""
        session = requests.Session()

        retry_strategy = Retry(
            total=3,
            backoff_factor=1,
            status_forcelist=[429, 500, 502, 503, 504],
            allowed_methods=["HEAD", "GET", "OPTIONS"],
        )

        adapter = HTTPAdapter(
            max_retries=retry_strategy,
            pool_connections=10,
            pool_maxsize=20,
        )

        session.mount("http://", adapter)
        session.mount("https://", adapter)

        session.headers.update(
            {
                "User-Agent": self.user_agent,
                "Accept": "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8",
                "Accept-Language": "de-DE,de;q=0.9,en;q=0.8",
            }
        )

        return session

    def parse_onvista_table(self, soup: BeautifulSoup, logger: logging.Logger) -> List[List[str]]:
        """Parst OnVista HTML-Tabelle"""

        # Finde Tabelle mit verschiedenen Strategien
        table = None
        strategies = [
            lambda s: s.find("table", class_="table"),
            lambda s: s.find("table", {"class": re.compile(r"table.*")}),
            lambda s: s.find("table"),
        ]

        for strategy in strategies:
            try:
                table = strategy(soup)
                if table:
                    rows = table.find_all("tr")
                    if len(rows) > 5:  # Mindestens Header + Daten
                        logger.info(f"✅ Tabelle gefunden mit {len(rows)} Zeilen")
                        break
            except Exception:
                continue

        if not table:
            logger.error("❌ Keine OnVista-Tabelle gefunden")
            return []

        # Extrahiere Daten
        raw_data = []
        rows = table.find_all("tr")

        for row in rows:
            cells = row.find_all(["td", "th"])
            if cells:
                row_data = [cell.get_text(strip=True) for cell in cells]
                raw_data.append(row_data)

        logger.info(f"📊 {len(raw_data) - 1} OnVista-Zertifikate extrahiert")
        return raw_data

    def fetch_onvista_data(self, logger: logging.Logger) -> Tuple[List[List[str]], str]:
        """Nur OnVista-Daten mit Selenium laden"""
        base_url = "https://www.onvista.de/derivate/Bonus-Zertifikate?"
        params = (
            "bonusYieldAskPctRange=7;60&"
            "cols=instrument,instrumentUnderlying.name,downBarrierAbs,dateMaturity,"
            "quote.bid,quote.ask,bonusYieldAskPct,bonusYieldPerAnnumAskPct,"
            "differenceDownBarrierPct,premiumAsk,bonus,spreadAskPct&"
            "dateMaturityRange=2025-06-13;2025-12-31&"
            "differenceBarrierPctRange=20;&"
            "feature=CLASSIC&"
            "idTypeSettlement=1&"
            "order=ASC&"
            "sort=premiumAsk"
        )
        target_url = base_url + params

        logger.info("🎯 Lade OnVista mit Selenium...")
        logger.info(f"🔗 URL: {target_url}")
        from selenium import webdriver
        from selenium.webdriver.chrome.options import Options
        from selenium.webdriver.common.by import By
        from selenium.webdriver.support import expected_conditions as EC
        from selenium.webdriver.support.ui import WebDriverWait

        chrome_options = Options()
        chrome_options.add_argument("--headless")
        chrome_options.add_argument("--no-sandbox")
        chrome_options.add_argument("--disable-dev-shm-usage")
        chrome_options.add_argument("--window-size=1920,1080")
        driver = webdriver.Chrome(options=chrome_options)
        try:
            driver.get(target_url)
            WebDriverWait(driver, 20).until(
                EC.presence_of_element_located((By.CSS_SELECTOR, "table"))
            )
            html = driver.page_source
            soup = BeautifulSoup(html, "html.parser")
            data = self.parse_onvista_table(soup, logger)
            return data, "ONVISTA"
        finally:
            driver.quit()

    def save_raw_data(
        self, data: List[List[str]], data_source: str, output_folder: Path, logger: logging.Logger
    ) -> Path:
        """Speichert Rohdaten als Excel-Datei"""

        if not data or len(data) < 2:
            raise ValueError("Keine Daten zum Speichern vorhanden")

        # DataFrame erstellen
        df = pd.DataFrame(data[1:], columns=data[0])

        # Datenquelle hinzufügen
        df["data_source"] = data_source
        df["fetch_timestamp"] = datetime.datetime.now()

        # Excel-Datei speichern
        timestamp = datetime.datetime.now().strftime("%Y-%m-%d_%H-%M-%S")
        excel_file = output_folder / f"raw_certificates_{data_source.lower()}_{timestamp}.xlsx"

        with pd.ExcelWriter(excel_file, engine="openpyxl") as writer:
            df.to_excel(writer, sheet_name="Raw_Data", index=False)

            # Metadaten-Sheet
            meta_df = pd.DataFrame(
                [
                    {
                        "data_source": data_source,
                        "fetch_timestamp": datetime.datetime.now(),
                        "record_count": len(df),
                        "columns": ", ".join(df.columns.tolist()),
                    }
                ]
            )
            meta_df.to_excel(writer, sheet_name="Metadata", index=False)

        logger.info(f"💾 Excel gespeichert: {excel_file.name}")
        logger.info(f"📊 {len(df)} Datensätze, Quelle: {data_source}")

        return excel_file


# ===============================================
# Gerald/ReportGenerator [Service]
# ===============================================
"""
Zertifikate Report Generator
Generiert strategische HTML-Reports aus analysierten Zertifikatsdaten.
Automatische Diversifikation durch Gruppierung nach Basiswerten.
"""

import sys
from pathlib import Path


class Config:
    """Zentrale Konfiguration"""

    BASE_OUTPUT_FOLDER = Path.home() / "Downloads" / "Zertifikatanalyse"
    AUTO_OPEN_HTML = True
    MAX_CERTIFICATES_PER_UNDERLYING = 2
    WATCHLIST_SIZE = 5


@service
class ReportGenerator(Service):
    """Hauptklasse für die Report-Generierung"""

    def find_latest_excel_file(self) -> Optional[Path]:
        """Findet die neueste analysierte Excel-Datei"""
        pattern = "analyzed_certificates_*.xlsx"
        files = list(self.output_folder.glob(pattern))

        if not files:
            return None

        # Neueste Datei nach Änderungszeit
        return max(files, key=os.path.getmtime)

    def diversify_by_underlying(self, df: pd.DataFrame) -> pd.DataFrame:
        """
        Diversifikation: Beschränkt Anzahl Zertifikate pro Basiswert
        Reduziert Klumpenrisiko und verbessert Portfolio-Balance
        """
        if df.empty:
            return df

        diversified_results = []

        for underlying in df["basiswert"].unique():
            underlying_df = df[df["basiswert"] == underlying].copy()

            # Beste Zertifikate je Basiswert nach Attractiveness Score
            top_certificates = underlying_df.sort_values(
                "attractiveness_score", ascending=False
            ).head(Config.MAX_CERTIFICATES_PER_UNDERLYING)

            diversified_results.append(top_certificates)

        # Zusammenfügen und nach Gesamtscore sortieren
        result_df = pd.concat(diversified_results, ignore_index=True)
        return result_df.sort_values("attractiveness_score", ascending=False)

    def create_html_table(self, df: pd.DataFrame, highlight_color: str) -> str:
        """Erstellt strukturierte HTML-Tabelle mit Basiswert-Gruppierung"""
        if df.empty:
            return "<p>Keine Daten verfügbar</p>"

        # Datenbereinigung
        df_clean = df.copy()
        if "attractiveness_score" in df_clean.columns:
            df_clean["attractiveness_score"] = df_clean["attractiveness_score"].round(0).astype(int)

        df_clean = df_clean.fillna(
            value=dict.fromkeys(df_clean.select_dtypes(include="object").columns, "N/A")
        )

        # Relevante Spalten für Investoren
        display_columns = [
            "wkn",
            "basiswert",
            "barriere",
            "bonus",
            "bewertungstag",
            "geld_kurs",
            "brief_kurs",
            "bonus_rendite_p_a",
            "abstand_barriere",
            "aufgeld",
            "spread",
            "attractiveness_score",
        ]

        available_columns = [col for col in display_columns if col in df_clean.columns]
        df_display = df_clean[available_columns]

        # Professionelle Spalten-Namen
        column_mapping = {
            "wkn": "WKN",
            "basiswert": "Basiswert",
            "barriere": "Barriere",
            "bonus": "Bonus",
            "bewertungstag": "Bewertung",
            "geld_kurs": "Geld",
            "brief_kurs": "Brief",
            "bonus_rendite_p_a": "Rendite p.a.",
            "abstand_barriere": "Barriere-Abstand",
            "aufgeld": "Aufgeld",
            "spread": "Spread",
            "attractiveness_score": "Score",
        }

        df_display = df_display.rename(columns=column_mapping)

        # HTML-Tabelle mit Gruppierung
        html = '<table border="1" cellspacing="0" cellpadding="4" style="border-collapse:collapse;width:100%;">'
        html += (
            "<thead><tr>"
            + "".join(f"<th>{col}</th>" for col in df_display.columns)
            + "</tr></thead><tbody>"
        )

        current_underlying = None
        for _, row in df_display.iterrows():
            underlying = row.get("Basiswert", "Unknown")

            # Gruppierungs-Header für neuen Basiswert
            if underlying != current_underlying:
                html += f'''<tr style="background-color:#f0f0f0;font-weight:bold;">
                    <td colspan="{len(df_display.columns)}" style="padding:8px;">
                        📊 {underlying}
                    </td>
                </tr>'''
                current_underlying = underlying

            # Zertifikat-Zeile
            html += f'<tr style="background-color:{highlight_color};">'
            html += "".join(f"<td>{row[col]}</td>" for col in df_display.columns)
            html += "</tr>"

        html += "</tbody></table>"
        return html

    def generate_info_boxes(self, strategy_count: int, watchlist_count: int) -> str:
        """Generiert Informations-Boxen für den Report"""
        bid_only_warning = """
        <div style="background-color: #fff3cd; border: 1px solid #ffeaa7; padding: 15px; margin: 20px 0; border-radius: 5px;">
            <h4 style="margin: 0 0 10px 0; color: #856404;">⚠️ Bid-Only Risiko</h4>
            <p style="margin: 0; color: #856404;">
                <strong>Manuelle Prüfung erforderlich:</strong> Prüfe vor dem Kauf auf 
                <a href="https://www.boerse-stuttgart.de/" target="_blank">Börse Stuttgart</a>, 
                ob das Zertifikat handelbar ist.
                <br><strong>Warnsignal:</strong> "BRIEF: -" oder "BidOnly" = nicht kaufbar!
            </p>
        </div>
        """

        diversification_info = f"""
        <div style="background-color: #e7f3ff; border: 1px solid #b3d9ff; padding: 15px; margin: 20px 0; border-radius: 5px;">
            <h4 style="margin: 0 0 10px 0; color: #0066cc;">📊 Diversifikations-Strategie</h4>
            <p style="margin: 0; color: #0066cc;">
                <strong>Automatische Diversifikation:</strong> Maximal {Config.MAX_CERTIFICATES_PER_UNDERLYING} 
                Zertifikate je Basiswert für optimale Risikostreuung.
                <br><strong>Vollständige Daten:</strong> Alle analysierten Zertifikate in der Excel-Datei verfügbar.
            </p>
        </div>
        """

        return bid_only_warning + diversification_info

    def generate_html_report(self, df: pd.DataFrame) -> Path:
        """Generiert den finalen HTML-Report"""
        timestamp = datetime.datetime.now().strftime("%Y-%m-%d")
        html_file = self.output_folder / f"zertifikate_report_{timestamp}.html"

        # Strategiekonforme Zertifikate
        strategy_raw = df[df["meets_strategy_criteria"]].copy()
        strategy_diversified = self.diversify_by_underlying(strategy_raw)

        # Watchlist-Kandidaten
        watchlist_raw = df[df.get("highlight", "") == "watchlist"].copy()
        watchlist_diversified = self.diversify_by_underlying(watchlist_raw)

        self.logger.info(
            f"✅ {len(strategy_raw)} strategiekonforme Zertifikate → {len(strategy_diversified)} nach Diversifikation"
        )
        self.logger.info(
            f"✅ {len(watchlist_raw)} Watchlist-Kandidaten → {len(watchlist_diversified)} nach Diversifikation"
        )

        # Statistiken loggen
        if not strategy_diversified.empty:
            underlying_stats = strategy_raw["basiswert"].value_counts()
            self.logger.info(f"📊 Verteilung Strategiekonforme: {dict(underlying_stats.head())}")

        # HTML-Report zusammenstellen
        html_content = f"""
        <!DOCTYPE html>
        <html lang="de">
        <head>
            <meta charset="UTF-8">
            <meta name="viewport" content="width=device-width, initial-scale=1.0">
            <title>Zertifikate Analyse Report</title>
            <style>
                body {{ font-family: 'Segoe UI', Arial, sans-serif; padding: 40px; line-height: 1.6; }}
                h1 {{ color: #2c3e50; border-bottom: 2px solid #3498db; padding-bottom: 10px; }}
                h2 {{ color: #34495e; margin-top: 30px; }}
                table {{ font-size: 0.9em; }}
                .timestamp {{ color: #7f8c8d; font-style: italic; }}
            </style>
        </head>
        <body>
            <h1>📊 Zertifikate Analyse Report</h1>
            <p class="timestamp">Generiert: {timestamp}</p>

            {self.generate_info_boxes(len(strategy_diversified), len(watchlist_diversified))}

            <h2>✅ Top Strategiekonforme Zertifikate</h2>
            <p>Beste {Config.MAX_CERTIFICATES_PER_UNDERLYING} Zertifikate je Basiswert nach Attractiveness Score</p>
            {self.create_html_table(strategy_diversified, "#d4edda")}

            <h2>🔍 Watchlist Kandidaten</h2>
            <p>Interessante Alternativen mit leicht erhöhtem Aufgeld</p>
            {self.create_html_table(watchlist_diversified, "#fff3cd")}

            <hr style="margin-top: 40px;">
            <p style="font-size: 0.9em; color: #7f8c8d;">
                <strong>Prozess:</strong> 1) WKN bei Börse Stuttgart eingeben → 2) Brief-Kurs prüfen → 3) Nur bei Bid+Ask kaufen
            </p>
        </body>
        </html>
        """

        # Report speichern
        with open(html_file, "w", encoding="utf-8") as f:
            f.write(html_content)

        self.logger.info(f"✅ Report gespeichert: {html_file}")

        # Optional: AI-Validierung hinzufügen
        self._try_ai_enhancement(html_file)

        # Automatisches Öffnen
        if Config.AUTO_OPEN_HTML:
            self._open_file(html_file)

        return html_file

    def _open_file(self, file_path: Path):
        """Öffnet Datei im Standard-Browser"""
        try:
            if sys.platform == "darwin":  # macOS
                os.system(f'open "{file_path}"')
            elif sys.platform == "win32":  # Windows
                os.startfile(str(file_path))
            else:  # Linux
                os.system(f'xdg-open "{file_path}"')
        except Exception as e:
            self.logger.warning(f"HTML konnte nicht automatisch geöffnet werden: {e}")

    def _run_ai_analysis(self, html_file: Path) -> Optional[str]:
        """Führt GPT-Analyse durch und gibt erweitertes HTML zurück"""
        try:
            with open(html_file, encoding="utf-8") as f:
                html_content = f.read()

            # Extrahiere beide Tabellen für GPT
            pattern_strategy = r"<h2>✅ Top Strategiekonforme Zertifikate</h2>.*?<table.*?</table>"
            pattern_watchlist = r"<h2>🔍 Watchlist Kandidaten</h2>.*?<table.*?</table>"

            strategy_match = re.search(pattern_strategy, html_content, re.DOTALL)
            watchlist_match = re.search(pattern_watchlist, html_content, re.DOTALL)

            if not strategy_match and not watchlist_match:
                self.logger.warning("❌ Keine geeigneten Tabellen für GPT gefunden")
                return None

            table_html = ""
            if strategy_match:
                table_html += "<h3>✅ Strategiekonforme Zertifikate</h3>\n" + strategy_match.group(
                    0
                )
            if watchlist_match:
                table_html += "<h3>🔍 Watchlist Kandidaten</h3>\n" + watchlist_match.group(0)

            # Prompt aus externer Datei laden und formatieren
            prompt_template_path = Path(
                "/Users/gcaesar/Library/CloudStorage/Dropbox/Dokumente_GC/Sixtyfour/Development/Zertifikatanalyse_Run/gpt_prompt_template.txt"
            )
            prompt_template = prompt_template_path.read_text(encoding="utf-8")
            html_wrapped = f"\n==== BEGINN HTML ====\n{table_html}\n==== ENDE HTML ===="
            prompt = prompt_template.format(table_html=html_wrapped)

            self.logger.info("⏳ GPT-Analyse wird durchgeführt...")

            # Neue OpenAI SDK >=1.0.0 Nutzung
            from openai import OpenAI

            client = OpenAI(
                api_key="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"
            )

            response = client.chat.completions.create(
                model="gpt-4o",
                messages=[
                    {"role": "system", "content": "Du bist ein erfahrener Finanzanalyst."},
                    {"role": "user", "content": prompt},
                ],
                temperature=0.3,
                timeout=60,
            )

            answer = response.choices[0].message.content.strip()
            # Entfernt GPT-Markdown-Codeblock-Markierungen (```html ... ```)
            answer = re.sub(r"^```html\s*", "", answer)
            answer = re.sub(r"\s*```$", "", answer)

            if not answer:
                self.logger.warning("⚠️ GPT-Antwort war leer")
                return None

            # Vorhandene GPT-Blöcke entfernen
            html_content = re.sub(
                r'<div class="ai-analysis">.*?<div class="ai-meta">.*?</div>\s*</div>',
                "",
                html_content,
                flags=re.DOTALL,
            )

            # GPT-Antwort einbetten
            timestamp = datetime.datetime.now().strftime("%Y-%m-%d %H:%M:%S")
            ai_block = f"""
<div class="ai-analysis" style="background:#f9f9fc;padding:24px;margin-top:40px;border-left:6px solid #4e5bdc;border-radius:8px;font-family:sans-serif;">
  <h2 style="color:#4e5bdc;font-size:1.6em;margin-top:0;">🤖 KI-Analyse & Empfehlung</h2>
  <div style="font-size:1em;line-height:1.6;color:#222;">
    {answer}
  </div>
  <div class="ai-meta" style="font-size:0.85em;color:#888;margin-top:16px;text-align:right;">
    Analyse erstellt: {timestamp} | Modell: GPT-4o
  </div>
</div>
</body>
</html>
"""

            html_content = re.sub(r"</body>\s*</html>", ai_block, html_content)

            return html_content

        except Exception as e:
            self.logger.warning(f"⚠️ GPT-Analyse fehlgeschlagen: {e}")
            return None

    def _try_ai_enhancement(self, html_file: Path):
        """Optional: KI-Analyse ergänzen, wenn GPT erreichbar"""
        html = self._run_ai_analysis(html_file)
        if html:
            with open(html_file, "w", encoding="utf-8") as f:
                f.write(html)
            self.logger.info("✅ GPT-Analyse in Report integriert")
        else:
            self.logger.info("ℹ️ GPT-Analyse nicht verfügbar – normaler Report bleibt erhalten")

    def prepare_watchlist(self, df: pd.DataFrame) -> pd.DataFrame:
        """Bereitet Watchlist-Kandidaten vor"""
        if "watchlist_candidate" not in df.columns:
            self.logger.warning("⚠️ Keine Watchlist-Kandidaten gefunden")
            df["highlight"] = ""
            return df

        watchlist_candidates = df[df["watchlist_candidate"]].copy()

        if watchlist_candidates.empty:
            self.logger.info("ℹ️ Keine Watchlist-Kandidaten vorhanden")
            df["highlight"] = ""
            return df

        # Top Watchlist nach Score
        top_watchlist = watchlist_candidates.sort_values(
            "attractiveness_score", ascending=False
        ).head(Config.WATCHLIST_SIZE)

        # Markierung setzen
        df["highlight"] = ""
        df.loc[df["wkn"].isin(top_watchlist["wkn"]), "highlight"] = "watchlist"

        self.logger.info(f"✅ {len(top_watchlist)} Watchlist-Kandidaten markiert")
        return df

    def run(self) -> bool:
        """Hauptprozess für Report-Generierung"""
        # Excel-Datei finden
        input_file = self.find_latest_excel_file()
        if not input_file:
            self.logger.error("❌ Keine analyzed_certificates-Datei gefunden")
            print("❌ Bitte zuerst Zertifikate analysieren lassen")
            return False

        self.logger.info(f"📂 Verwende Datei: {input_file.name}")

        try:
            # Daten laden
            df = pd.read_excel(input_file, sheet_name="Analyzed_Data")

            # Watchlist vorbereiten
            df = self.prepare_watchlist(df)

            # Report generieren
            report_file = self.generate_html_report(df)

            print(f"✅ Report erfolgreich erstellt: {report_file}")
            return True

        except Exception as e:
            self.logger.error(f"❌ Fehler bei Report-Generierung: {e}")
            print(f"❌ Fehler: {e}")
            return False
