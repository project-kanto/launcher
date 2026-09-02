import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { open } from "@tauri-apps/plugin-dialog";
import { openUrl } from "@tauri-apps/plugin-opener";

type Platform = "android" | "ios";

interface Manifest {
  release_version?: string;
  source_url?: string;
}

interface Dashboard {
  host: string;
  environment: "development" | "production";
  server?: { status: string; version: string };
  android?: Manifest;
  ios?: Manifest;
  cached_android_original: boolean;
  cached_ios_original: boolean;
  supports_32_bit_apps: boolean;
}

interface SourceCheck {
  valid: boolean;
  message: string;
}

interface PreparedBuild {
  path: string;
  release_version: string;
}

interface InstallResponse {
  started: boolean;
  needs_permission: boolean;
}

interface IosSetup {
  devices: { name: string; udid: string }[];
  signed_in_as: string | null;
}

interface TwoFactorPrompt {
  method: "device" | "sms";
  phones: { id: number; last_two_digits: string }[];
}

interface InstallFinished {
  success: boolean;
  message: string;
}

const app = document.querySelector<HTMLDivElement>("#app")!;
app.innerHTML = `
  <div class="shell">
    <aside class="rail">
      <button class="brand" data-view="home" aria-label="Kanto Launcher home">
        <svg class="brand-mark" viewBox="0 0 32 32" aria-hidden="true">
          <path class="mark-page" d="M7 0h18a7 7 0 0 1 7 7v16l-9 9H7a7 7 0 0 1-7-7V7a7 7 0 0 1 7-7z"/>
          <path class="mark-fold" d="M32 23l-9 9v-9z"/>
          <g class="mark-letter" fill="none" stroke-width="5.4"><path d="M9.6 6.4v19.2"/><path d="M22.6 6.4L11.2 16l7.8 6.6"/></g>
        </svg>
      </button>

      <nav class="primary-nav" aria-label="Launcher">
        <button class="nav-item active" data-view="home" aria-current="page">
          <svg viewBox="0 0 24 24" aria-hidden="true"><path d="m4 10 8-6 8 6v9a1 1 0 0 1-1 1h-5v-6h-4v6H5a1 1 0 0 1-1-1z"/></svg>
          <span>Home</span>
        </button>
        <button class="nav-item" data-view="library">
          <svg viewBox="0 0 24 24" aria-hidden="true"><path d="m8 5 11 7-11 7z"/></svg>
          <span>Play</span>
        </button>
      </nav>

      <div class="rail-footer">
        <span id="environment" class="environment">Local</span>
        <span class="rail-version" id="rail-version">…</span>
      </div>
    </aside>

    <main class="stage">
      <section class="page home-page" data-page="home">
        <header class="page-bar">
          <div><span>Welcome back</span><strong>Kanto Launcher</strong></div>
          <div id="server-status" class="status-pill status-loading" role="status"><span></span>Checking Kanto</div>
        </header>

        <div class="home-content">
          <section class="home-hero">
            <div class="hero-copy">
              <span class="eyebrow">THE PRESERVED 2016 WORLD</span>
              <h1>Adventure,<br>the way you remember it.</h1>
              <p>Kanto brings the original mobile adventure back online, with a growing world and a launcher that handles the fiddly bits.</p>
              <button class="hero-action" data-view="library">Play Kanto <span aria-hidden="true">→</span></button>
            </div>
            <div class="hero-scene" aria-hidden="true">
              <span class="orbit orbit-one"></span>
              <span class="orbit orbit-two"></span>
              <span class="planet"></span>
              <span class="hero-mark">
                <svg viewBox="0 0 32 32"><path class="mark-page" d="M7 0h18a7 7 0 0 1 7 7v16l-9 9H7a7 7 0 0 1-7-7V7a7 7 0 0 1 7-7z"/><path class="mark-fold" d="M32 23l-9 9v-9z"/><g class="mark-letter" fill="none" stroke-width="5.4"><path d="M9.6 6.4v19.2"/><path d="M22.6 6.4L11.2 16l7.8 6.6"/></g></svg>
              </span>
            </div>
          </section>

          <div class="home-lower">
            <div class="quick-links">
              <button class="quick-link server-link" data-view="library">
                <span class="quick-icon"><span class="live-dot"></span></span>
                <span><small>Game server</small><strong id="home-server-label">Checking status…</strong></span>
                <span aria-hidden="true">→</span>
              </button>
              <a class="quick-link support-link" href="https://kanto.ac/support" target="_blank" rel="noopener noreferrer external">
                <span class="quick-icon">♥</span>
                <span><small>Keep Kanto running</small><strong>Support the project</strong></span>
                <span aria-hidden="true">↗</span>
              </a>
            </div>

            <section class="news" aria-labelledby="news-heading">
              <div class="section-heading"><span>Latest news</span><a href="https://kanto.ac/community" target="_blank" rel="noopener noreferrer external">Community ↗</a></div>
              <a class="news-lead" href="https://kanto.ac/community" target="_blank" rel="noopener noreferrer external">
                <div><span>Current release</span><strong id="news-version">Kanto is checking for updates</strong><small>The latest game build and server status, together in one place.</small></div>
                <span aria-hidden="true">→</span>
              </a>
              <div class="news-row"><span>Launcher preview</span><strong>A simpler route from download to your phone</strong><small>Now</small></div>
            </section>
          </div>
        </div>
      </section>

      <section class="page library-page" data-page="library" hidden>
        <header class="page-bar library-bar">
          <div><span>Play</span><strong id="workspace-heading">Kanto</strong></div>
        </header>

        <div class="library-content">
          <ol class="journey" aria-label="Installation progress">
            <li data-stage="prepare" class="current"><span>1 of 3</span><strong>Get ready</strong></li>
            <li data-stage="connect"><span>2 of 3</span><strong>Connect your phone</strong></li>
            <li data-stage="install"><span>3 of 3</span><strong>Install Kanto</strong></li>
          </ol>

          <section class="home-view">
            <div class="game-details">
              <p class="overline">PLAY KANTO</p>
              <h1>Choose your phone</h1>
              <p class="intro">We’ll guide you through installing Kanto from start to finish.</p>

              <div class="devices" aria-label="Choose your phone">
                <article class="device" data-card="android">
                  <span class="platform-icon">Android</span>
                  <div class="device-copy"><h2>Android phone</h2><p>Install Kanto directly on your phone.</p><p id="android-compatibility" class="compatibility" hidden></p></div>
                  <div class="device-action"><span class="availability" id="android-version">Checking…</span><button data-start="android" disabled>Install</button></div>
                </article>
                <article class="device" data-card="ios">
                  <span class="platform-icon">iOS</span>
                  <div class="device-copy"><h2>iPhone or iPad</h2><p>Connect your device with a cable. We’ll handle the rest.</p></div>
                  <div class="device-action"><span class="availability" id="ios-version">Checking…</span><button data-start="ios" disabled>Install</button></div>
                </article>
              </div>
            </div>
          </section>

          <section id="setup" class="setup" hidden aria-live="polite">
            <button class="back" data-close-setup aria-label="Back to Play">← Play</button>
            <div class="step-heading"><h2 id="setup-title">Choose Pokémon GO</h2></div>
            <p id="setup-copy" class="step-copy">Choose your Pokémon GO app file to begin. We’ll check that it works with Kanto before continuing.</p>
            <p id="source-result" class="result" role="status"></p>
            <div class="actions">
              <button id="download-original" hidden>Download Pokémon GO</button>
              <button id="choose-original" class="secondary">Choose Pokémon GO file</button>
              <button id="prepare-selected" hidden>Continue</button>
              <button id="install-android" hidden>Install Kanto</button>
            </div>
          </section>

          <section id="ios-install" class="setup install-step" hidden>
            <button class="back" data-close-setup aria-label="Back to Play">← Play</button>
            <div class="step-heading"><h2>Connect your iPhone</h2></div>
            <p class="step-copy">Unlock your iPhone, connect it with a cable, and tap Trust if asked. Your Apple password is never saved.</p>
            <div class="field-row">
              <label>Connected device<select id="ios-device"><option value="">No device found</option></select></label>
              <button id="refresh-ios" class="secondary">Refresh</button>
            </div>
            <form id="apple-login-fields" class="fields">
              <label>Apple ID<input id="apple-email" type="email" autocomplete="username" placeholder="you@example.com" required></label>
              <label>Password<input id="apple-password" type="password" autocomplete="current-password" required></label>
              <button id="apple-login">Sign in securely</button>
            </form>
            <div id="apple-session-row" class="session-row" hidden>
              <p id="apple-session" class="signed-in"></p>
              <button id="change-apple-id" class="secondary">Change</button>
            </div>
            <form id="two-factor" class="two-factor" hidden>
              <label>Apple verification code<input id="apple-code" inputmode="numeric" autocomplete="one-time-code" minlength="6" maxlength="6" pattern="[0-9]{6}" required></label>
              <button id="submit-apple-code">Verify code</button>
              <div id="sms-options" class="actions"></div>
            </form>
            <p id="ios-result" class="result" role="status"></p>
            <div class="install-actions">
              <button id="developer-mode-help" class="secondary">Developer Mode help</button>
              <button id="install-ios" hidden>Install Kanto</button>
            </div>
          </section>
        </div>
      </section>

      <p id="load-error" class="load-error" role="alert" hidden>Couldn’t reach Kanto. Check your connection and retry.</p>
    </main>

    <dialog id="developer-mode-guide" class="guide-dialog" aria-labelledby="developer-mode-title">
      <form method="dialog" class="guide-panel">
        <button class="guide-close" aria-label="Close Developer Mode guide">×</button>
        <div class="guide-heading">
          <span>Finish on your iPhone</span>
          <h2 id="developer-mode-title">Turn on Developer Mode</h2>
          <p>Your iPhone may ask for this before Kanto can open. It only takes a minute.</p>
        </div>
        <div class="guide-images">
          <button type="button" class="guide-shot" data-guide-image="/guides/developer-mode-settings.webp" aria-label="Enlarge the Settings screenshot">
            <img src="/guides/developer-mode-settings.webp" alt="iPhone Settings showing Privacy &amp; Security and the Developer Mode option.">
            <span>Enlarge</span>
          </button>
          <button type="button" class="guide-shot" data-guide-image="/guides/developer-mode-restart.webp" aria-label="Enlarge the restart screenshot">
            <img src="/guides/developer-mode-restart.webp" alt="iPhone Developer Mode switch and the Restart confirmation.">
            <span>Enlarge</span>
          </button>
          <button type="button" class="guide-shot" data-guide-image="/guides/developer-mode-confirm.webp" aria-label="Enlarge the confirmation screenshot">
            <img src="/guides/developer-mode-confirm.webp" alt="iPhone confirmation and passcode screens shown after restarting.">
            <span>Enlarge</span>
          </button>
        </div>
        <ol class="guide-steps">
          <li><strong>Find Developer Mode</strong><span>Open Settings → Privacy &amp; Security, scroll down, then tap Developer Mode.</span></li>
          <li><strong>Restart your iPhone</strong><span>Turn Developer Mode on, then tap Restart when your iPhone asks.</span></li>
          <li><strong>Confirm after restart</strong><span>Unlock your iPhone, tap Enable, and enter your passcode if asked.</span></li>
        </ol>
        <div class="guide-footer">
          <p>Can’t see Developer Mode? Finish installing Kanto first, keep your iPhone connected, then check again. Your screens may look slightly different on newer iOS versions.</p>
          <button value="done">Got it</button>
        </div>
      </form>
    </dialog>

    <dialog id="guide-image-viewer" class="guide-viewer" aria-label="Enlarged Developer Mode screenshot">
      <form method="dialog">
        <button class="guide-close" aria-label="Close enlarged screenshot">×</button>
        <img alt="">
      </form>
    </dialog>
  </div>`;

let dashboard: Dashboard | undefined;
let activePlatform: Platform = "android";
let selectedPath: string | undefined;
let preparedPath: string | undefined;

type Stage = "prepare" | "connect" | "install";

function showView(view: "home" | "library") {
  document.querySelectorAll<HTMLElement>("[data-page]").forEach((page) => {
    page.hidden = page.dataset.page !== view;
  });
  document.querySelectorAll<HTMLButtonElement>(".nav-item").forEach((item) => {
    const active = item.dataset.view === view;
    item.classList.toggle("active", active);
    if (active) item.setAttribute("aria-current", "page");
    else item.removeAttribute("aria-current");
  });
  if (view === "library") closeSetup();
}

function setStage(stage: Stage) {
  const stages: Stage[] = ["prepare", "connect", "install"];
  const current = stages.indexOf(stage);
  document.querySelectorAll<HTMLElement>("[data-stage]").forEach((item) => {
    const index = stages.indexOf(item.dataset.stage as Stage);
    item.classList.toggle("current", index === current);
    item.classList.toggle("complete", index < current);
  });
}

function closeSetup() {
  app.classList.remove("setup-active");
  document.querySelector<HTMLElement>("#setup")!.hidden = true;
  document.querySelector<HTMLElement>("#ios-install")!.hidden = true;
  document.querySelector("#workspace-heading")!.textContent = "Kanto";
  setStage("prepare");
}

function manifest(platform: Platform): Manifest | undefined {
  return dashboard?.[platform];
}

function hasCachedOriginal(platform: Platform): boolean {
  return platform === "android"
    ? Boolean(dashboard?.cached_android_original)
    : Boolean(dashboard?.cached_ios_original);
}

function showSetup(platform: Platform) {
  showView("library");
  activePlatform = platform;
  app.classList.add("setup-active");
  setStage("prepare");
  document.querySelector("#workspace-heading")!.textContent =
    platform === "android" ? "Android" : "iPhone or iPad";
  const setup = document.querySelector<HTMLElement>("#setup")!;
  const source = manifest(platform)?.source_url;
  const cached = hasCachedOriginal(platform);
  document.querySelector("#setup-title")!.textContent = cached ? "Pokémon GO is ready" : "Choose Pokémon GO";
  document.querySelector("#setup-copy")!.textContent = cached
    ? "Your verified game file is saved on this device. You won’t need to choose it again."
    : "Choose your Pokémon GO app file to begin. We’ll check that it works with Kanto before continuing.";
  const download = document.querySelector<HTMLButtonElement>("#download-original")!;
  download.hidden = !source && !cached;
  download.textContent = cached ? "Continue" : "Download Pokémon GO";
  document.querySelector<HTMLButtonElement>("#choose-original")!.classList.toggle("secondary", Boolean(source) || cached);
  selectedPath = undefined;
  preparedPath = undefined;
  document.querySelector<HTMLButtonElement>("#prepare-selected")!.hidden = true;
  document.querySelector<HTMLButtonElement>("#install-android")!.hidden = true;
  document.querySelector<HTMLElement>("#ios-install")!.hidden = true;
  document.querySelector("#source-result")!.textContent = "";
  setup.hidden = false;
}

async function chooseOriginal() {
  const path = await open({
    multiple: false,
    directory: false,
    filters: [{ name: "Original game", extensions: [activePlatform === "android" ? "apk" : "ipa"] }],
  });
  if (!path) return;
  const result = document.querySelector<HTMLElement>("#source-result")!;
  result.className = "result checking";
  result.textContent = "Checking file…";
  try {
    const check = await invoke<SourceCheck>("verify_original", { path, platform: activePlatform });
    result.className = `result ${check.valid ? "good" : "bad"}`;
    result.textContent = check.message;
    selectedPath = check.valid ? path : undefined;
    document.querySelector<HTMLButtonElement>("#prepare-selected")!.hidden = !check.valid;
    document.querySelector<HTMLButtonElement>("#choose-original")!.classList.toggle("secondary", check.valid || Boolean(manifest(activePlatform)?.source_url));
  } catch (error) {
    result.className = "result bad";
    result.textContent = String(error);
  }
}

async function prepare(sourcePath?: string) {
  const result = document.querySelector<HTMLElement>("#source-result")!;
  const buttons = document.querySelectorAll<HTMLButtonElement>("#setup button");
  buttons.forEach((button) => (button.disabled = true));
  result.className = "result checking";
  result.textContent = sourcePath
    ? "Preparing Kanto…"
    : hasCachedOriginal(activePlatform)
      ? "Preparing your saved game…"
      : "Downloading and preparing Kanto…";
  try {
    const prepared = await invoke<PreparedBuild>("prepare_release", {
      platform: activePlatform,
      sourcePath,
    });
    result.className = "result good";
    preparedPath = prepared.path;
    if (sourcePath && dashboard) {
      if (activePlatform === "android") dashboard.cached_android_original = true;
      else dashboard.cached_ios_original = true;
    }
    if (activePlatform === "android" && dashboard?.host === "android") {
      result.textContent = `Kanto ${prepared.release_version} is ready. Tap Install Kanto to finish.`;
      document.querySelector<HTMLButtonElement>("#install-android")!.hidden = false;
      setStage("install");
    } else {
      result.textContent = `Kanto ${prepared.release_version} is ready. Connect your iPhone to finish installing.`;
      document.querySelector<HTMLElement>("#setup")!.hidden = true;
      document.querySelector<HTMLElement>("#ios-install")!.hidden = false;
      setStage("connect");
      await loadIosSetup();
    }
  } catch (error) {
    result.className = "result bad";
    result.textContent = String(error);
  } finally {
    buttons.forEach((button) => (button.disabled = false));
  }
}

async function loadIosSetup() {
  const result = document.querySelector<HTMLElement>("#ios-result")!;
  const select = document.querySelector<HTMLSelectElement>("#ios-device")!;
  result.className = "result checking";
  result.textContent = "Looking for your iPhone…";
  try {
    const setup = await invoke<IosSetup>("load_ios_setup");
    select.replaceChildren(
      ...(setup.devices.length
        ? setup.devices.map((device) => new Option(device.name, device.udid))
        : [new Option("No device found", "")]),
    );
    const sessionRow = document.querySelector<HTMLElement>("#apple-session-row")!;
    const session = document.querySelector<HTMLElement>("#apple-session")!;
    const loginFields = document.querySelector<HTMLElement>("#apple-login-fields")!;
    const install = document.querySelector<HTMLButtonElement>("#install-ios")!;
    if (setup.signed_in_as) {
      session.textContent = `Signed in securely as ${setup.signed_in_as}`;
      sessionRow.hidden = false;
      loginFields.hidden = true;
      install.hidden = false;
    } else {
      sessionRow.hidden = true;
      loginFields.hidden = false;
      install.hidden = true;
    }
    result.className = setup.devices.length ? "result good" : "result bad";
    result.textContent = setup.devices.length
      ? setup.signed_in_as
        ? "Device connected. Tap Sign and install Kanto."
        : "Device connected. Sign in, then Kanto can install the game."
      : "No iPhone found. Unlock it, connect by USB, tap Trust, then refresh.";
  } catch (error) {
    result.className = "result bad";
    result.textContent = String(error);
  }
}

async function appleLogin() {
  const result = document.querySelector<HTMLElement>("#ios-result")!;
  const email = document.querySelector<HTMLInputElement>("#apple-email")!.value;
  const passwordInput = document.querySelector<HTMLInputElement>("#apple-password")!;
  const password = passwordInput.value;
  passwordInput.value = "";
  const button = document.querySelector<HTMLButtonElement>("#apple-login")!;
  button.disabled = true;
  result.className = "result checking";
  result.textContent = "Signing in with Apple…";
  try {
    const signedInAs = await invoke<string>("apple_login", { email, password });
    const sessionRow = document.querySelector<HTMLElement>("#apple-session-row")!;
    const session = document.querySelector<HTMLElement>("#apple-session")!;
    session.textContent = `Signed in securely as ${signedInAs}`;
    sessionRow.hidden = false;
    document.querySelector<HTMLElement>("#apple-login-fields")!.hidden = true;
    document.querySelector<HTMLElement>("#two-factor")!.hidden = true;
    document.querySelector<HTMLButtonElement>("#install-ios")!.hidden = false;
    result.className = "result good";
    result.textContent = "Apple ID ready. Kanto can now sign and install the game.";
  } catch (error) {
    result.className = "result bad";
    result.textContent = String(error);
  } finally {
    button.disabled = false;
  }
}

async function respondToApple(code?: string, phoneId?: number) {
  const result = document.querySelector<HTMLElement>("#ios-result")!;
  try {
    await invoke("respond_apple_2fa", { code, phoneId });
    result.className = "result checking";
    result.textContent = phoneId ? "Asking Apple to send a text message…" : "Checking verification code…";
    document.querySelector<HTMLElement>("#two-factor")!.hidden = true;
  } catch (error) {
    result.className = "result bad";
    result.textContent = String(error);
  }
}

async function installIos() {
  const result = document.querySelector<HTMLElement>("#ios-result")!;
  const button = document.querySelector<HTMLButtonElement>("#install-ios")!;
  const udid = document.querySelector<HTMLSelectElement>("#ios-device")!.value;
  if (!preparedPath || !udid) {
    result.className = "result bad";
    result.textContent = "Connect and select an iPhone first.";
    return;
  }
  button.disabled = true;
  setStage("install");
  result.className = "result checking";
  result.textContent = "Starting the secure signing process…";
  try {
    await invoke("sign_and_install_ios", { path: preparedPath, udid });
  } catch (error) {
    button.disabled = false;
    result.className = "result bad";
    result.textContent = String(error);
  }
}

async function installAndroid() {
  if (!preparedPath) return;
  const result = document.querySelector<HTMLElement>("#source-result")!;
  try {
    const install = await invoke<InstallResponse>("install_prepared", { path: preparedPath });
    result.className = "result good";
    result.textContent = install.needs_permission
      ? "Allow Kanto to install apps, come back here, then tap Install Kanto again."
      : "Android’s installer is open. Confirm Install to finish.";
  } catch (error) {
    result.className = "result bad";
    result.textContent = String(error);
  }
}

async function load() {
  try {
    dashboard = await invoke<Dashboard>("load_dashboard");
    document.querySelector("#environment")!.textContent =
      dashboard.environment === "development" ? "DEV" : "LIVE";
    document.querySelector("#environment")!.className = `environment environment-${dashboard.environment}`;
    document.querySelector("#rail-version")!.textContent =
      dashboard.server ? `v${dashboard.server.version}` : "Offline";
    document.querySelector("#news-version")!.textContent = dashboard.server
      ? `Kanto ${dashboard.server.version} is ready`
      : "Kanto is currently unavailable";
    const status = document.querySelector<HTMLElement>("#server-status")!;
    const reported = dashboard.server?.status ?? "offline";
    const state = ["online", "degraded", "maintenance", "offline"].includes(reported)
      ? reported
      : "offline";
    status.className = `status-pill status-${state}`;
    status.replaceChildren(document.createElement("span"), `${state[0].toUpperCase()}${state.slice(1)}`);
    document.querySelector("#home-server-label")!.textContent =
      state === "online" ? `Online · Kanto ${dashboard.server?.version ?? ""}` : `${state[0].toUpperCase()}${state.slice(1)}`;

    for (const platform of ["android", "ios"] as const) {
      const release = dashboard[platform];
      document.querySelector(`#${platform}-version`)!.textContent = release
        ? `Kanto ${release.release_version ?? dashboard.server?.version ?? "available"}`
        : "Not available";
      document.querySelector<HTMLButtonElement>(`[data-start="${platform}"]`)!.disabled = !release;
    }
    if (dashboard.host === "android") {
      document.querySelector<HTMLElement>('[data-card="ios"]')!.hidden = true;
      if (!dashboard.supports_32_bit_apps) {
        const compatibility = document.querySelector<HTMLElement>("#android-compatibility")!;
        compatibility.hidden = false;
        compatibility.textContent =
          "This phone can’t run 32-bit apps, so Kanto won’t install here. You can try it inside a VPhone-style environment.";
        document.querySelector<HTMLButtonElement>('[data-start="android"]')!.disabled = true;
      }
    } else {
      document.querySelector<HTMLElement>('[data-card="android"]')!.hidden = true;
    }
  } catch {
    document.querySelector<HTMLElement>("#load-error")!.hidden = false;
    const status = document.querySelector<HTMLElement>("#server-status")!;
    status.className = "status-pill status-offline";
    status.replaceChildren(document.createElement("span"), "Offline");
    document.querySelector("#home-server-label")!.textContent = "Server unavailable";
  }
}

document.querySelectorAll<HTMLElement>("[data-view]").forEach((item) =>
  item.addEventListener("click", (event) => {
    if (item instanceof HTMLAnchorElement) return;
    event.preventDefault();
    showView(item.dataset.view as "home" | "library");
  }),
);
document.querySelectorAll<HTMLAnchorElement>('a[href^="https://kanto.ac/"]').forEach((link) =>
  link.addEventListener("click", (event) => {
    event.preventDefault();
    openUrl(link.href).catch(() => {
      const error = document.querySelector<HTMLElement>("#load-error")!;
      error.textContent = "Couldn’t open that Kanto page in your browser.";
      error.hidden = false;
    });
  }),
);
document.querySelectorAll<HTMLButtonElement>("[data-start]").forEach((button) =>
  button.addEventListener("click", () => showSetup(button.dataset.start as Platform)),
);
document.querySelector("#choose-original")!.addEventListener("click", chooseOriginal);
document.querySelector("#download-original")!.addEventListener("click", () => prepare());
document.querySelector("#prepare-selected")!.addEventListener("click", () => prepare(selectedPath));
document.querySelector("#install-android")!.addEventListener("click", installAndroid);
document.querySelector("#refresh-ios")!.addEventListener("click", loadIosSetup);
document.querySelector("#apple-login-fields")!.addEventListener("submit", (event) => {
  event.preventDefault();
  appleLogin();
});
document.querySelector("#change-apple-id")!.addEventListener("click", () => {
  document.querySelector<HTMLElement>("#apple-session-row")!.hidden = true;
  document.querySelector<HTMLElement>("#apple-login-fields")!.hidden = false;
  document.querySelector<HTMLButtonElement>("#install-ios")!.hidden = true;
  document.querySelector<HTMLInputElement>("#apple-email")!.focus();
});
document.querySelector("#two-factor")!.addEventListener("submit", (event) => {
  event.preventDefault();
  respondToApple(document.querySelector<HTMLInputElement>("#apple-code")!.value);
});
document.querySelector("#install-ios")!.addEventListener("click", installIos);
document.querySelector("#developer-mode-help")!.addEventListener("click", () => {
  document.querySelector<HTMLDialogElement>("#developer-mode-guide")!.showModal();
});
const guideViewer = document.querySelector<HTMLDialogElement>("#guide-image-viewer")!;
const guideViewerImage = guideViewer.querySelector<HTMLImageElement>("img")!;
document.querySelectorAll<HTMLButtonElement>("[data-guide-image]").forEach((button) => {
  button.addEventListener("click", () => {
    const thumbnail = button.querySelector<HTMLImageElement>("img")!;
    guideViewerImage.src = button.dataset.guideImage!;
    guideViewerImage.alt = thumbnail.alt;
    guideViewer.showModal();
  });
});
guideViewer.addEventListener("click", (event) => {
  if (event.target === guideViewer) guideViewer.close();
});
document.querySelectorAll("[data-close-setup]").forEach((button) =>
  button.addEventListener("click", closeSetup),
);

listen<TwoFactorPrompt>("apple-2fa-required", ({ payload }) => {
  const panel = document.querySelector<HTMLElement>("#two-factor")!;
  const result = document.querySelector<HTMLElement>("#ios-result")!;
  const sms = document.querySelector<HTMLElement>("#sms-options")!;
  sms.replaceChildren();
  for (const phone of payload.phones) {
    const button = document.createElement("button");
    button.type = "button";
    button.className = "secondary";
    button.textContent = `Text ••${phone.last_two_digits} instead`;
    button.addEventListener("click", () => respondToApple(undefined, phone.id));
    sms.append(button);
  }
  panel.hidden = false;
  result.className = "result checking";
  result.textContent = payload.method === "sms"
    ? "Enter the code Apple sent by text message."
    : "Enter the code shown on your trusted Apple device.";
  document.querySelector<HTMLInputElement>("#apple-code")!.focus();
});

listen<string>("ios-install-progress", ({ payload }) => {
  const result = document.querySelector<HTMLElement>("#ios-result")!;
  result.className = "result checking";
  result.textContent = payload;
});

listen<InstallFinished>("ios-install-finished", ({ payload }) => {
  const result = document.querySelector<HTMLElement>("#ios-result")!;
  result.className = `result ${payload.success ? "good" : "bad"}`;
  result.textContent = payload.message;
  document.querySelector<HTMLButtonElement>("#install-ios")!.disabled = false;
  if (payload.success) {
    document.querySelectorAll<HTMLElement>("[data-stage]").forEach((item) => {
      item.classList.remove("current");
      item.classList.add("complete");
    });
    if (!localStorage.getItem("kanto.developerModeGuideSeen")) {
      document.querySelector<HTMLDialogElement>("#developer-mode-guide")!.showModal();
      localStorage.setItem("kanto.developerModeGuideSeen", "1");
    }
  }
});

load();
