function $(id) { return document.getElementById(id); }

const urlInput = $('urlInput');
const pasteBtn = $('pasteBtn');
const downloadBtn = $('downloadBtn');
const browseBtn = $('browseBtn');
const savePathText = $('savePathText');
const status = $('status');
const result = $('result');
const resultText = $('resultText');
const revealBtn = $('revealBtn');
const progressArea = $('progressArea');
const progressBar = $('progressBar');
const progressText = $('progressText');

let downloadedPath = null;
let saveDir = null; // null = 默认 Desktop

function T() { return window.__TAURI__; }

function getHome() {
  return navigator.userAgent.includes('Mac') ? '/Users' + (navigator.platform.includes('Mac') ? '' : '') : '';
}

// 更新进度显示
function setProgress(percent, text) {
  progressBar.value = percent;
  progressText.textContent = percent > 0 ? percent + '%' : '';
  if (text) {
    status.textContent = text;
    status.className = 'status loading';
  }
}

function showProgress(show) {
  progressArea.classList.toggle('active', show);
}

// 监听下载进度事件
let unlistenProgress = null;
async function setupProgressListener() {
  if (unlistenProgress) {
    await unlistenProgress();
  }
  unlistenProgress = await T().event.listen('download-progress', (event) => {
    const { percent, status: st } = event.payload;
    setProgress(percent, st);
  });
}

// 读取剪贴板
pasteBtn.addEventListener('click', async () => {
  try {
    const text = await navigator.clipboard.readText();
    urlInput.value = text.trim();
    status.textContent = '';
    status.className = 'status';
    result.classList.add('hidden');
  } catch (e) {
    status.textContent = '无法读取剪贴板，请手动粘贴';
    status.className = 'status error';
  }
});

// 浏览选择保存路径
browseBtn.addEventListener('click', async () => {
  try {
    const selected = await T().core.invoke('select_folder');
    if (selected) {
      saveDir = selected;
      // 显示最后一段路径
      const parts = selected.split('/');
      const display = parts.length > 2 ? '…/' + parts.slice(-2).join('/') : selected;
      savePathText.textContent = display;
      savePathText.title = selected;
    }
  } catch (e) {
    status.textContent = '选择文件夹失败';
    status.className = 'status error';
  }
});

// 下载
downloadBtn.addEventListener('click', async () => {
  const url = urlInput.value.trim();
  if (!url) {
    status.textContent = '请先粘贴小红书链接';
    status.className = 'status error';
    return;
  }

  downloadBtn.disabled = true;
  status.textContent = '';
  status.className = 'status';
  result.classList.add('hidden');
  setProgress(0, '准备...');
  showProgress(true);

  try {
    await setupProgressListener();
    const res = await T().core.invoke('download_xhs', { url, saveDir });
    if (res.success) {
      status.textContent = '';
      status.className = 'status success';
      resultText.textContent = res.message;
      result.classList.remove('hidden');
      downloadedPath = res.path || null;
      setProgress(100, '完成');
    } else {
      status.textContent = res.message;
      status.className = 'status error';
      showProgress(false);
    }
  } catch (e) {
    status.textContent = '出错: ' + e;
    status.className = 'status error';
    showProgress(false);
  } finally {
    downloadBtn.disabled = false;
  }
});

// 在 Finder 中显示
revealBtn.addEventListener('click', () => {
  if (downloadedPath) {
    T().core.invoke('open_in_finder', { path: downloadedPath });
  }
});

// 自动读取剪贴板（如果内容是链接）
window.addEventListener('DOMContentLoaded', async () => {
  try {
    const text = await navigator.clipboard.readText();
    if (text && text.includes('xhslink')) {
      urlInput.value = text.trim();
    }
  } catch (e) {
    // ignore
  }
});
