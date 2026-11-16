let mailWindow;
const windowName = "orange-mailbox";

function mailboxPath(path) {
  if (path !== "/mailbox/") throw new Error("邮件入口配置错误");
  return path;
}

export async function openMailWindow(api) {
  if (mailWindow && !mailWindow.closed) {
    mailWindow.focus();
    return;
  }

  const opened = window.open(
    "",
    windowName,
    "popup,width=1280,height=850,resizable=yes,scrollbars=yes",
  );
  if (!opened) throw new Error("邮件窗口被拦截，请允许弹出窗口");
  mailWindow = opened;
  opened.focus();
  if (opened.location.href !== "about:blank") return;

  try {
    const status = await api("/admin/mail/status");
    mailboxPath(status.entry_path);
    if (!status.enabled) throw new Error(status.error || "邮件服务尚未配置");
    const session = await api("/admin/mail/session", { method: "POST" });
    const path = mailboxPath(session.entry_path);
    if (opened.closed) {
      if (mailWindow === opened) mailWindow = null;
      return;
    }
    const form = document.createElement("form");
    form.method = "POST";
    form.action = path + "?_task=login&_action=plugin.orange_sso";
    form.target = windowName;
    const ticket = document.createElement("input");
    ticket.type = "hidden";
    ticket.name = "_ticket";
    ticket.value = session.ticket;
    form.append(ticket);
    document.body.append(form);
    form.submit();
    form.remove();
  } catch (failure) {
    if (!opened.closed) opened.close();
    if (mailWindow === opened) mailWindow = null;
    throw failure;
  }
}
