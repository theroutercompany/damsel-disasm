const state = {
  imageId: null,
  fileName: null,
  architecture: null,
  entryPoint: null,
  sections: [],
  defaultSection: null,
  disasm: null,
  selectedInstructionIndex: 0,
  symbolLookupTimer: null,
};

const elements = {
  body: document.body,
  dropzone: document.getElementById("dropzone"),
  fileInput: document.getElementById("file-input"),
  fileSummary: document.getElementById("file-summary"),
  banner: document.getElementById("banner"),
  sectionSelect: document.getElementById("section-select"),
  symbolInput: document.getElementById("symbol-input"),
  symbolSuggestions: document.getElementById("symbol-suggestions"),
  addressInput: document.getElementById("address-input"),
  fromInput: document.getElementById("from-input"),
  toInput: document.getElementById("to-input"),
  bytesInput: document.getElementById("bytes-input"),
  limitInput: document.getElementById("limit-input"),
  annotationsToggle: document.getElementById("annotations-toggle"),
  valueFlowToggle: document.getElementById("value-flow-toggle"),
  analysisToggle: document.getElementById("analysis-toggle"),
  runButton: document.getElementById("run-button"),
  resultTitle: document.getElementById("result-title"),
  resultMeta: document.getElementById("result-meta"),
  instructionRows: document.getElementById("instruction-rows"),
  inspectorTitle: document.getElementById("inspector-title"),
  inspectorContent: document.getElementById("inspector-content"),
  sectionGroup: document.getElementById("section-group"),
  symbolGroup: document.getElementById("symbol-group"),
  addressGroup: document.getElementById("address-group"),
  listingPane: document.querySelector(".canvas-pane"),
  inspectorPane: document.querySelector(".inspector-pane"),
  controlsPane: document.querySelector(".controls-pane"),
};

function currentTargetKind() {
  return document.querySelector('input[name="target-kind"]:checked').value;
}

function setBanner(message, tone = "error") {
  if (!message) {
    elements.banner.hidden = true;
    elements.banner.textContent = "";
    elements.banner.dataset.tone = "";
    return;
  }

  elements.banner.hidden = false;
  elements.banner.textContent = message;
  elements.banner.dataset.tone = tone;
}

function syncWorkbenchState() {
  const selectedInstruction = state.disasm?.instructions?.[state.selectedInstructionIndex];
  elements.body.dataset.hasImage = state.imageId ? "true" : "false";
  elements.body.dataset.hasSelection = selectedInstruction ? "true" : "false";
}

function setWorkbenchPhase(phase) {
  elements.body.dataset.phase = phase;
}

function pulseSurface(node) {
  if (!node) {
    return;
  }

  node.classList.remove("is-refreshing");
  void node.offsetWidth;
  node.classList.add("is-refreshing");
  window.setTimeout(() => {
    node.classList.remove("is-refreshing");
  }, 220);
}

function updateFileSummary() {
  const label = elements.fileSummary.querySelector(".summary-label");
  const value = elements.fileSummary.querySelector(".summary-value");

  if (!state.imageId) {
    label.textContent = "Idle";
    value.textContent = "Awaiting local binary";
    return;
  }

  label.textContent = state.fileName;
  value.textContent = `${state.architecture} · entry ${formatAddress(state.entryPoint)} · ${state.sections.length} sections`;
}

function updateTargetControls() {
  const targetKind = currentTargetKind();
  elements.sectionGroup.hidden = targetKind !== "section";
  elements.symbolGroup.hidden = targetKind !== "symbol";
  elements.addressGroup.hidden = targetKind !== "address";
  elements.fromInput.disabled = targetKind === "address";
}

function formatAddress(value) {
  if (value === null || value === undefined) {
    return "n/a";
  }

  const bigint = BigInt(value);
  return `0x${bigint.toString(16)}`;
}

function numericValue(input) {
  if (!input.value.trim()) {
    return null;
  }

  const value = Number(input.value);
  return Number.isFinite(value) ? value : null;
}

function currentStartAddressHint(targetKind) {
  if (targetKind === "address" && elements.addressInput.value.trim()) {
    try {
      return Number.parseInt(elements.addressInput.value.trim().replace(/^0x/i, ""), 16);
    } catch {
      return null;
    }
  }

  if (targetKind === "section") {
    const section = state.sections.find((entry) => entry.name === elements.sectionSelect.value);
    return section ? Number(section.address) : null;
  }

  if (elements.fromInput.value.trim()) {
    return numericValue(elements.fromInput);
  }

  return null;
}

function validateInputs() {
  const targetKind = currentTargetKind();
  if (!state.imageId) {
    return "Upload a Mach-O before requesting disassembly.";
  }

  if (targetKind === "section" && !elements.sectionSelect.value) {
    return "Choose a section target.";
  }

  if (targetKind === "symbol" && !elements.symbolInput.value.trim()) {
    return "Enter a symbol name.";
  }

  if (targetKind === "address" && !elements.addressInput.value.trim()) {
    return "Enter a decode address.";
  }

  const bytes = numericValue(elements.bytesInput);
  const limit = numericValue(elements.limitInput);
  const from = numericValue(elements.fromInput);
  const to = numericValue(elements.toInput);
  const startHint = currentStartAddressHint(targetKind);

  if (bytes !== null && bytes <= 0) {
    return "Bytes must be greater than 0.";
  }

  if (limit !== null && limit <= 0) {
    return "Limit must be greater than 0.";
  }

  if (bytes !== null && limit !== null) {
    return "Bytes cannot be combined with limit.";
  }

  if (to !== null && (bytes !== null || limit !== null)) {
    return "To cannot be combined with bytes or limit.";
  }

  if (from !== null && to !== null && to <= from) {
    return "To must be greater than from.";
  }

  if (startHint !== null && to !== null && to <= startHint) {
    return "To must be greater than the decode start.";
  }

  return null;
}

function buildDisasmRequest() {
  const targetKind = currentTargetKind();
  const window = {
    from: targetKind === "address" ? undefined : numericValue(elements.fromInput),
    to: numericValue(elements.toInput),
    bytes: numericValue(elements.bytesInput),
    limit: numericValue(elements.limitInput),
  };
  const value =
    targetKind === "section"
      ? elements.sectionSelect.value
      : targetKind === "symbol"
        ? elements.symbolInput.value.trim()
        : elements.addressInput.value.trim();

  return {
    target: {
      kind: targetKind,
      value,
    },
    window,
    options: {
      includeAnnotations: elements.annotationsToggle.checked,
      includeValueFlow: elements.valueFlowToggle.checked,
      includeAnalysis: elements.analysisToggle.checked,
    },
  };
}

async function loadImage(file) {
  setBanner("");
  setWorkbenchPhase("uploading");
  pulseSurface(elements.controlsPane);

  const formData = new FormData();
  formData.append("file", file, file.name || "uploaded-macho");

  try {
    const response = await fetch("/api/images", {
      method: "POST",
      body: formData,
    });

    if (!response.ok) {
      return handleApiError(response);
    }

    const payload = await response.json();
    state.imageId = payload.imageId;
    state.fileName = payload.fileName;
    state.architecture = payload.architecture;
    state.entryPoint = payload.entryPoint;
    state.sections = payload.sections || [];
    state.defaultSection = payload.defaultSection;
    populateSections();
    updateFileSummary();
    syncWorkbenchState();

    if (payload.defaultSection) {
      elements.sectionSelect.value = payload.defaultSection;
    }

    pulseSurface(elements.controlsPane);
    await runDisasm();
  } finally {
    setWorkbenchPhase("idle");
  }
}

function populateSections() {
  elements.sectionSelect.innerHTML = "";
  for (const section of state.sections) {
    const option = document.createElement("option");
    option.value = section.name;
    option.textContent = `${section.name}${section.executable ? " · exec" : ""}`;
    elements.sectionSelect.append(option);
  }
}

async function fetchSymbolSuggestions() {
  if (!state.imageId || !elements.symbolInput.value.trim()) {
    elements.symbolSuggestions.innerHTML = "";
    return;
  }

  const query = new URLSearchParams({
    q: elements.symbolInput.value.trim(),
    limit: "20",
  });
  const response = await fetch(`/api/images/${state.imageId}/symbols?${query.toString()}`);
  if (!response.ok) {
    return;
  }

  const payload = await response.json();
  elements.symbolSuggestions.innerHTML = "";
  for (const symbol of payload.symbols || []) {
    const option = document.createElement("option");
    option.value = symbol;
    elements.symbolSuggestions.append(option);
  }
}

async function runDisasm() {
  const validation = validateInputs();
  if (validation) {
    setBanner(validation);
    return;
  }

  setBanner("");
  setWorkbenchPhase("decoding");

  try {
    const response = await fetch(`/api/images/${state.imageId}/disasm`, {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
      },
      body: JSON.stringify(buildDisasmRequest()),
    });

    if (!response.ok) {
      return handleApiError(response);
    }

    state.disasm = await response.json();
    state.selectedInstructionIndex = 0;
    renderDisasm();
    renderInspector();
    syncWorkbenchState();
    pulseSurface(elements.listingPane);
    pulseSurface(elements.inspectorPane);
  } finally {
    setWorkbenchPhase("idle");
  }
}

async function handleApiError(response) {
  const payload = await response.json().catch(() => null);
  const message = payload
    ? `${payload.code}: ${payload.message}`
    : `Request failed with status ${response.status}`;
  setBanner(message);
}

function renderDisasm() {
  const data = state.disasm;
  if (!data || !Array.isArray(data.instructions) || data.instructions.length === 0) {
    elements.resultTitle.textContent = "No target";
    elements.resultMeta.textContent = "Load a local binary to decode a target.";
    elements.instructionRows.innerHTML = `
      <tr class="empty-row">
        <td colspan="3">This target did not produce any decoded instructions.</td>
      </tr>
    `;
    return;
  }

  elements.resultTitle.textContent = data.target;
  const analysisMeta = data.analysis
    ? ` · ${data.analysis.summary.basic_block_count} blocks · ${data.analysis.summary.edge_count} edges · ${data.analysis.summary.import_count} imports`
    : "";
  elements.resultMeta.textContent = `${data.instruction_count} rows · ${data.stop_reason}${analysisMeta}`;
  elements.instructionRows.innerHTML = "";

  data.instructions.forEach((instruction, index) => {
    const row = document.createElement("tr");
    row.classList.toggle("is-selected", index === state.selectedInstructionIndex);
    row.addEventListener("click", () => {
      state.selectedInstructionIndex = index;
      renderDisasm();
      renderInspector();
      syncWorkbenchState();
    });

    const targetRef = (instruction.references || []).find(
      (reference) =>
        (reference.type === "call" || reference.type === "branch") &&
        reference.target !== undefined,
    );
    const jumpChip = targetRef
      ? `<button class="instruction-target" type="button" data-target="${targetRef.target}">jump ${formatAddress(targetRef.target)}</button>`
      : "";

    row.innerHTML = `
      <td class="instruction-address mono">${formatAddress(instruction.address)}</td>
      <td class="instruction-opcode mono">${formatAddress(instruction.opcode)}</td>
      <td>
        <div class="instruction-main">
          <div class="instruction-rendered mono">${escapeHtml(instruction.rendered)}</div>
          ${jumpChip}
        </div>
      </td>
    `;

    const jumpButton = row.querySelector("[data-target]");
    if (jumpButton) {
      jumpButton.addEventListener("click", async (event) => {
        event.stopPropagation();
        await jumpToAddress(Number(jumpButton.dataset.target));
      });
    }

    elements.instructionRows.append(row);
  });
}

function renderInspector() {
  const data = state.disasm;
  const instruction = data?.instructions?.[state.selectedInstructionIndex];
  if (!instruction) {
    elements.inspectorTitle.textContent = "No selection";
    elements.inspectorContent.innerHTML =
      '<p class="inspector-empty">Select an instruction to inspect references and recovered values.</p>';
    return;
  }

  elements.inspectorTitle.textContent = `${instruction.mnemonic} · ${formatAddress(
    instruction.address,
  )}`;

  const blocks = [];
  blocks.push(
    renderDetailBlock("Instruction", [
      ["Address", formatAddress(instruction.address)],
      ["Opcode", formatAddress(instruction.opcode)],
      ["Rendered", instruction.rendered],
    ]),
  );
  blocks.push(renderReferenceBlock(instruction.references || []));
  blocks.push(renderObjectBlock("Annotations", instruction.annotations || []));
  blocks.push(renderObjectBlock("Recovered Values", instruction.recovered_values || []));
  if (data.analysis) {
    blocks.push(renderAnalysisBlock(data.analysis));
  }
  elements.inspectorContent.innerHTML = blocks.join("");

  elements.inspectorContent
    .querySelectorAll("[data-jump-target]")
    .forEach((button) => {
      button.addEventListener("click", async () => {
        await jumpToAddress(Number(button.dataset.jumpTarget));
      });
    });
}

function renderAnalysisBlock(analysis) {
  return `
    <section class="inspector-block analysis-block">
      <h3>Summary Graph</h3>
      <div class="detail-list">
        ${renderDetailRows([
          ["Blocks", analysis.summary.basic_block_count],
          ["Edges", analysis.summary.edge_count],
          ["Calls", analysis.summary.direct_call_count],
          ["Indirect Calls", analysis.summary.indirect_call_count],
          ["Branches", analysis.summary.branch_count],
          ["Returns", analysis.summary.return_count],
          ["Imports", analysis.summary.import_count],
          ["Cache Links", analysis.summary.cache_link_count],
          ["Recovered Values", analysis.summary.recovered_value_count],
          ["Jump Tables", analysis.summary.jump_table_count],
        ])}
      </div>
      ${renderAnalysisBlocks(analysis.basic_blocks || [])}
      ${renderAnalysisEdges(analysis.edges || [])}
      ${renderAnalysisImports(analysis.imports || [], analysis.cache_links || [])}
    </section>
  `;
}

function renderDetailBlock(title, rows) {
  return `
    <section class="inspector-block">
      <h3>${title}</h3>
      <div class="detail-list">
        ${renderDetailRows(rows)}
      </div>
    </section>
  `;
}

function renderDetailRows(rows) {
  return rows
    .map(
      ([key, value]) => `
        <div class="detail-row">
          <span class="detail-key">${escapeHtml(key)}</span>
          <span class="mono">${escapeHtml(String(value))}</span>
        </div>
      `,
    )
    .join("");
}

function renderAnalysisBlocks(blocks) {
  if (!blocks.length) {
    return "";
  }

  return `
    <div class="analysis-list">
      <div class="item-eyebrow">Basic Blocks</div>
      ${blocks
        .map(
          (block) => `
            <div class="analysis-row">
              <span class="mono">#${escapeHtml(block.id)}</span>
              ${renderJumpButton(block.start_address)}
              <span class="mono">${escapeHtml(formatAddress(block.end_address))}</span>
              <span class="mono">${escapeHtml(String(block.instruction_count))} insn</span>
            </div>
          `,
        )
        .join("")}
    </div>
  `;
}

function renderAnalysisEdges(edges) {
  if (!edges.length) {
    return "";
  }

  return `
    <div class="analysis-list">
      <div class="item-eyebrow">Analysis Edges</div>
      ${edges
        .map(
          (edge) => `
            <div class="analysis-row">
              <span class="mono">${escapeHtml(edge.kind)}</span>
              ${renderJumpButton(edge.source_address)}
              <span class="detail-key">to</span>
              ${
                edge.target_address === null
                  ? '<span class="mono">return</span>'
                  : renderJumpButton(edge.target_address)
              }
            </div>
          `,
        )
        .join("")}
    </div>
  `;
}

function renderAnalysisImports(imports, cacheLinks) {
  if (!imports.length && !cacheLinks.length) {
    return "";
  }

  return `
    <div class="analysis-list">
      <div class="item-eyebrow">Imports</div>
      ${imports
        .map(
          (entry) => `
            <div class="analysis-row">
              ${renderJumpButton(entry.instruction_address)}
              <span class="mono">${escapeHtml(entry.dylib)}</span>
              <span class="mono">${escapeHtml(entry.name)}</span>
            </div>
          `,
        )
        .join("")}
      ${cacheLinks
        .map(
          (entry) => `
            <div class="analysis-row">
              ${renderJumpButton(entry.instruction_address)}
              <span class="mono">${escapeHtml(entry.provider_kind)}</span>
              <span class="mono">${escapeHtml(entry.provider_install_name)}</span>
            </div>
          `,
        )
        .join("")}
    </div>
  `;
}

function renderJumpButton(value) {
  return `
    <button class="jump-button mono" type="button" data-jump-target="${value}">
      ${escapeHtml(formatAddress(value))}
    </button>
  `;
}

function renderReferenceBlock(references) {
  if (!references.length) {
    return renderEmptyBlock("References", "No references recorded for this instruction.");
  }

  return `
    <section class="inspector-block">
      <h3>References</h3>
      <div class="data-list">
        ${references
          .map((reference) => {
            const details = Object.entries(reference)
              .filter(([key]) => key !== "type")
              .map(([key, value]) => {
                if ((key === "target" || key.endsWith("_address")) && typeof value === "number") {
                  return `
                    <div class="detail-row">
                      <span class="detail-key">${escapeHtml(key)}</span>
                      <button class="jump-button mono" type="button" data-jump-target="${value}">
                        ${escapeHtml(formatAddress(value))}
                      </button>
                    </div>
                  `;
                }

                return `
                  <div class="detail-row">
                    <span class="detail-key">${escapeHtml(key)}</span>
                    <span class="mono">${escapeHtml(formatValue(value))}</span>
                  </div>
                `;
              })
              .join("");

            return `
              <div class="data-item">
                <div class="item-eyebrow">${escapeHtml(reference.type || "reference")}</div>
                <div class="detail-list">${details}</div>
              </div>
            `;
          })
          .join("")}
      </div>
    </section>
  `;
}

function renderObjectBlock(title, entries) {
  if (!entries.length) {
    return renderEmptyBlock(title, `No ${title.toLowerCase()} for the selected instruction.`);
  }

  return `
    <section class="inspector-block">
      <h3>${escapeHtml(title)}</h3>
      <div class="data-list">
        ${entries
          .map(
            (entry) => `
              <div class="data-item">
                ${Object.entries(entry)
                  .map(
                    ([key, value]) => `
                      <div class="detail-row">
                        <span class="detail-key">${escapeHtml(key)}</span>
                        <span class="mono">${escapeHtml(formatValue(value))}</span>
                      </div>
                    `,
                  )
                  .join("")}
              </div>
            `,
          )
          .join("")}
      </div>
    </section>
  `;
}

function renderEmptyBlock(title, message) {
  return `
    <section class="inspector-block">
      <h3>${escapeHtml(title)}</h3>
      <p class="inspector-empty">${escapeHtml(message)}</p>
    </section>
  `;
}

async function jumpToAddress(target) {
  document.querySelector('input[name="target-kind"][value="address"]').checked = true;
  updateTargetControls();
  elements.addressInput.value = formatAddress(target);
  elements.fromInput.value = "";
  const to = numericValue(elements.toInput);
  if (to !== null && to <= target) {
    elements.toInput.value = "";
  }
  await runDisasm();
}

function formatValue(value) {
  if (value === null || value === undefined) {
    return "null";
  }

  if (typeof value === "number") {
    return Number.isInteger(value) ? formatAddress(value) : String(value);
  }

  if (typeof value === "boolean") {
    return value ? "true" : "false";
  }

  return String(value);
}

function escapeHtml(value) {
  return String(value)
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll('"', "&quot;")
    .replaceAll("'", "&#39;");
}

function installUploadHandlers() {
  elements.fileInput.addEventListener("change", async (event) => {
    const [file] = event.target.files || [];
    if (file) {
      await loadImage(file);
    }
  });

  ["dragenter", "dragover"].forEach((eventName) => {
    elements.dropzone.addEventListener(eventName, (event) => {
      event.preventDefault();
      elements.dropzone.classList.add("drag-over");
    });
  });

  ["dragleave", "drop"].forEach((eventName) => {
    elements.dropzone.addEventListener(eventName, (event) => {
      event.preventDefault();
      elements.dropzone.classList.remove("drag-over");
    });
  });

  elements.dropzone.addEventListener("drop", async (event) => {
    const [file] = event.dataTransfer?.files || [];
    if (file) {
      await loadImage(file);
    }
  });
}

function installEventHandlers() {
  installUploadHandlers();
  document.querySelectorAll('input[name="target-kind"]').forEach((input) => {
    input.addEventListener("change", updateTargetControls);
  });
  elements.runButton.addEventListener("click", runDisasm);
  elements.symbolInput.addEventListener("input", () => {
    clearTimeout(state.symbolLookupTimer);
    state.symbolLookupTimer = window.setTimeout(fetchSymbolSuggestions, 180);
  });
}

updateTargetControls();
updateFileSummary();
renderDisasm();
renderInspector();
syncWorkbenchState();
installEventHandlers();
