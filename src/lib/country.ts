export function iso2ToFlag(iso2: string): string {
  if (!iso2 || iso2.length !== 2 || !/^[A-Z]{2}$/.test(iso2)) {
    return "🏳";
  }
  const base = 0x1f1e6;
  return String.fromCodePoint(
    ...[...iso2].map((c) => base + c.charCodeAt(0) - 65),
  );
}

const NAMES: Record<string, string> = {
  HK: "Hong Kong",
  JP: "Japan",
  SG: "Singapore",
  TW: "Taiwan",
  US: "United States",
  GB: "United Kingdom",
  DE: "Germany",
  KR: "South Korea",
  FR: "France",
  MY: "Malaysia",
  CN: "China",
  CA: "Canada",
  AU: "Australia",
  NL: "Netherlands",
  IN: "India",
  BR: "Brazil",
  RU: "Russia",
  TR: "Turkey",
  AE: "UAE",
  IL: "Israel",
  IT: "Italy",
  ES: "Spain",
  CH: "Switzerland",
  SE: "Sweden",
  NO: "Norway",
  FI: "Finland",
  DK: "Denmark",
  PL: "Poland",
  UA: "Ukraine",
  MX: "Mexico",
  AR: "Argentina",
  TH: "Thailand",
  PH: "Philippines",
  VN: "Vietnam",
  ID: "Indonesia",
  ZA: "South Africa",
  XX: "Unknown",
};

export function iso2ToDisplay(iso2: string): string {
  return NAMES[iso2] ?? iso2;
}
