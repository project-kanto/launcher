import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { open } from "@tauri-apps/plugin-dialog";

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
    <aside class="sidebar">
      <a class="brand" href="#" aria-label="Kanto Launcher home">
        <svg class="brand-mark" viewBox="0 0 32 32" aria-hidden="true">
          <path class="mark-page" d="M7 0h18a7 7 0 0 1 7 7v16l-9 9H7a7 7 0 0 1-7-7V7a7 7 0 0 1 7-7z"/>
          <path class="mark-fold" d="M32 23l-9 9v-9z"/>
          <g class="mark-letter" fill="none" stroke-width="5.4"><path d="M9.6 6.4v19.2"/><path d="M22.6 6.4L11.2 16l7.8 6.6"/></g>
        </svg>
        <span><strong>Kanto</strong><small>Launcher</small></span>
      </a>

      <div class="client-summary">
        <span class="client-icon" aria-hidden="true">K</span>
        <div><small>Game client</small><strong id="sidebar-version">Checking…</strong></div>
      </div>

      <ol class="journey" aria-label="Installation progress">
        <li data-stage="prepare" class="current"><span>1</span><div><strong>Prepare</strong><small>Verify and patch</small></div></li>
        <li data-stage="connect"><span>2</span><div><strong>Connect</strong><small>Phone and account</small></div></li>
        <li data-stage="install"><span>3</span><div><strong>Install</strong><small>Sign and finish</small></div></li>
      </ol>

      <div class="sidebar-footer">
        <span id="environment" class="environment">Local build</span>
        <div id="server-status" class="status status-loading" role="status"><span></span>Checking servers</div>
      </div>
    </aside>

    <main class="workspace">
      <header class="workspace-bar">
        <div><span>Installer</span><strong id="workspace-heading">Kanto setup</strong></div>
        <span class="secure-note">Verified locally</span>
      </header>

      <section class="home-view">
        <div class="home-main">
          <p class="overline">READY TO INSTALL</p>
          <h1>Kanto for your phone</h1>
          <p class="intro">Choose the device you’re setting up. Kanto will prepare the correct game build and guide the rest.</p>

          <div class="devices" aria-label="Choose your phone">
            <article class="device" data-card="android">
              <span class="platform-icon">Android</span>
              <div class="device-copy"><h2>Android phone</h2><p>Prepare and install directly on this device.</p><p id="android-compatibility" class="compatibility" hidden></p></div>
              <div class="device-action"><span class="availability" id="android-version">Checking…</span><button data-start="android" disabled>Set up</button></div>
            </article>
            <article class="device" data-card="ios">
              <span class="platform-icon">iOS</span>
              <div class="device-copy"><h2>iPhone or iPad</h2><p>Connect by USB, then Kanto signs and installs it.</p></div>
              <div class="device-action"><span class="availability" id="ios-version">Checking…</span><button data-start="ios" disabled>Set up</button></div>
            </article>
          </div>
        </div>

        <aside class="release-info" aria-label="What Kanto Launcher does">
          <p class="overline">THIS INSTALL</p>
          <ul>
            <li><span>01</span><div><strong>Original verified</strong><small>Wrong files are refused before anything changes.</small></div></li>
            <li><span>02</span><div><strong>Patched on-device</strong><small>Your original game file stays on this computer.</small></div></li>
            <li><span>03</span><div><strong>Guided install</strong><small>Every phone confirmation is explained as it appears.</small></div></li>
          </ul>
        </aside>
      </section>

      <section id="setup" class="setup" hidden aria-live="polite">
        <button class="back" data-close-setup aria-label="Back to device selection">← Back</button>
        <div class="step-heading"><span>1</span><div><p class="overline">PREPARE</p><h2 id="setup-title">Check the original game</h2></div></div>
        <p class="step-copy">Kanto only accepts the exact supported version. The file is checked and patched locally.</p>
        <div class="actions">
          <button id="download-original" hidden>Download and prepare</button>
          <button id="choose-original" class="secondary">Choose original file</button>
          <button id="prepare-selected" hidden>Prepare Kanto</button>
          <button id="install-android" hidden>Install Kanto</button>
        </div>
        <p id="source-result" class="result" role="status"></p>
      </section>

      <section id="ios-install" class="setup install-step" hidden>
        <button class="back" data-close-setup aria-label="Back to device selection">← Back</button>
        <div class="step-heading"><span>2</span><div><p class="overline">CONNECT &amp; INSTALL</p><h2>Finish on your iPhone</h2></div></div>
        <p class="step-copy">Unlock your device, connect it by USB, and tap Trust if asked. Your Apple password is never saved.</p>
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
        <button id="install-ios" class="install-button" hidden>Sign and install Kanto</button>
        <p id="ios-result" class="result" role="status"></p>
      </section>

      <p id="load-error" class="load-error" role="alert" hidden>Couldn’t reach Kanto. Check your connection and retry.</p>
    </main>
  </div>`;

let dashboard: Dashboard | undefined;
let activePlatform: Platform = "android";
let selectedPath: string | undefined;
let preparedPath: string | undefined;

type Stage = "prepare" | "connect" | "install";

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
  document.querySelector("#workspace-heading")!.textContent = "Kanto setup";
  setStage("prepare");
}

function manifest(platform: Platform): Manifest | undefined {
  return dashboard?.[platform];
}

function showSetup(platform: Platform) {
  activePlatform = platform;
  app.classList.add("setup-active");
  setStage("prepare");
  document.querySelector("#workspace-heading")!.textContent =
    platform === "android" ? "Android setup" : "iPhone setup";
  const setup = document.querySelector<HTMLElement>("#setup")!;
  document.querySelector("#setup-title")!.textContent =
    platform === "android" ? "Check the original Android game" : "Check the original iPhone game";
  const source = manifest(platform)?.source_url;
  const download = document.querySelector<HTMLButtonElement>("#download-original")!;
  download.hidden = !source;
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
  result.textContent = sourcePath ? "Preparing Kanto…" : "Downloading and preparing Kanto…";
  try {
    const prepared = await invoke<PreparedBuild>("prepare_release", {
      platform: activePlatform,
      sourcePath,
    });
    result.className = "result good";
    preparedPath = prepared.path;
    if (activePlatform === "android" && dashboard?.host === "android") {
      result.textContent = `Kanto ${prepared.release_version} is prepared and verified. Tap Install Kanto.`;
      document.querySelector<HTMLButtonElement>("#install-android")!.hidden = false;
      setStage("install");
    } else {
      result.textContent = `Kanto ${prepared.release_version} is prepared and verified. Signing is next.`;
      await loadIosSetup();
      document.querySelector<HTMLElement>("#setup")!.hidden = true;
      document.querySelector<HTMLElement>("#ios-install")!.hidden = false;
      setStage("connect");
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
      dashboard.environment === "development" ? "Development build" : "Production";
    document.querySelector("#environment")!.className = `environment environment-${dashboard.environment}`;
    document.querySelector("#sidebar-version")!.textContent =
      dashboard.server ? `Kanto ${dashboard.server.version}` : "Unavailable";
    const status = document.querySelector<HTMLElement>("#server-status")!;
    const reported = dashboard.server?.status ?? "offline";
    const state = ["online", "degraded", "maintenance", "offline"].includes(reported)
      ? reported
      : "offline";
    status.className = `status status-${state}`;
    status.replaceChildren(document.createElement("span"), `${state[0].toUpperCase()}${state.slice(1)}`);

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
    status.className = "status status-offline";
    status.replaceChildren(document.createElement("span"), "Offline");
  }
}

document.querySelectorAll<HTMLButtonElement>("[data-start]").forEach((button) =>
  button.addEventListener("click", () => showSetup(button.dataset.start as Platform)),
);
document.querySelector(".brand")!.addEventListener("click", (event) => {
  event.preventDefault();
  closeSetup();
});
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
  }
});

load();
