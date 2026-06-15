import type { Route, GlobalSettings } from "../types";

class AppStore {
  currentRoute = $state<Route>("project");
  globalSettings = $state<GlobalSettings>({ nordvpn_path: null });

  navigate(to: Route): void {
    this.currentRoute = to;
  }

  setNordvpnPath(path: string | null): void {
    this.globalSettings.nordvpn_path = path;
  }

  updateGlobalSettings(settings: GlobalSettings): void {
    this.globalSettings = settings;
  }
}

export const appStore = new AppStore();
