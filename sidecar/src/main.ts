import { buscar, buscar_legacy, SearchConfig } from "./scrapper";

function parseArgs(): SearchConfig {
  const args = process.argv.slice(2);
  const config: SearchConfig = {
    lat: 0,
    lng: 0,
    searchTerm: "",
    zoom: 0,
  };

  for (let i = 0; i < args.length; i++) {
    switch (args[i]) {
      case "--lat":
        config.lat = parseFloat(args[++i]);
        break;
      case "--lng":
        config.lng = parseFloat(args[++i]);
        break;
      case "--search":
        config.searchTerm = args[++i];
        break;
      case "--browser":
        config.browserPath = args[++i];
        break;
      case "--zoom":
        config.zoom = parseInt(args[++i], 10);
        break;
      case "--ua":
        config.userAgent = args[++i];
        break;
      case "--threshold":
        config.stopThreshold = parseInt(args[++i], 10);
        break;
    }
  }

  return config;
}

async function main() {
  const config = parseArgs();

  if (!config.searchTerm || (!config.lat && config.lat !== 0) || (!config.lng && config.lng !== 0)) {
    console.error("Uso: scrape --lat <lat> --lng <lng> --search <termino> [--browser <path>] [--ua <ua>] [--threshold <n>]");
    process.exit(1);
  }

  await buscar(config);
  process.exit(0);
}

main().catch((err) => {
  console.log(JSON.stringify({ error: err instanceof Error ? err.message : String(err) }));
  process.exit(1);
});
