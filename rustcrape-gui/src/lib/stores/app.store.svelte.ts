import type { Route, GlobalSettings, Announcement } from "../types";

class AppStore {
  currentRoute = $state<Route>("project");
  globalSettings = $state<GlobalSettings>({ nordvpn_path: null, browser_path: null });
  announcement = $state<Announcement | null>(null);

  navigate(to: Route): void {
    this.currentRoute = to;
  }

  setNordvpnPath(path: string | null): void {
    this.globalSettings.nordvpn_path = path;
  }

  setBrowserPath(path: string | null): void {
    this.globalSettings.browser_path = path;
  }

  updateGlobalSettings(settings: GlobalSettings): void {
    this.globalSettings = settings;
  }
}

export const appStore = new AppStore();
