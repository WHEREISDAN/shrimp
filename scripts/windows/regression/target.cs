// The app the Windows regression walkthrough dictates into: one window with a
// multi-line text box, a text field, a password field and a list. Built as
// ShrimpRegressionTarget.exe by run.ps1, so Shrimp sees it as its own app.
// The controls are the system's own EDIT and LISTBOX, as in Notepad, so UI
// Automation reads them the way Shrimp meets them in most Windows apps.
//
// The helper drives it with the registered window message
// "ShrimpRegressionTarget": wParam 1–4 focuses the text box, the field, the
// password field or the list (a control that isn't a text field), 9 quits,
// 101–104 return those controls' windows, 105 returns which one has focus,
// and 106 returns how the last paste came (1 Ctrl+V, 2 Shift+Insert) and
// forgets it.

using System;
using System.Drawing;
using System.Reflection;
using System.Runtime.InteropServices;
using System.Windows.Forms;

[assembly: AssemblyTitle("Shrimp Regression Target")]
[assembly: AssemblyProduct("Shrimp Regression Target")]

class Target : Form, IMessageFilter
{
    [DllImport("user32.dll", CharSet = CharSet.Unicode)]
    static extern int RegisterWindowMessage(string name);

    [DllImport("user32.dll", CharSet = CharSet.Unicode)]
    static extern IntPtr CreateWindowEx(int exStyle, string className, string title, int style,
        int x, int y, int width, int height, IntPtr parent, IntPtr menu, IntPtr instance, IntPtr param);

    [DllImport("user32.dll")] static extern IntPtr SetFocus(IntPtr hwnd);
    [DllImport("user32.dll")] static extern IntPtr GetFocus();
    [DllImport("user32.dll")] static extern short GetKeyState(int key);
    [DllImport("user32.dll")] static extern IntPtr SendMessage(IntPtr hwnd, int message, IntPtr wParam, IntPtr lParam);

    const int WS_CHILD = 0x40000000, WS_VISIBLE = 0x10000000, WS_BORDER = 0x00800000, WS_TABSTOP = 0x00010000;
    const int WS_VSCROLL = 0x00200000, WS_HSCROLL = 0x00100000;
    const int ES_MULTILINE = 0x0004, ES_PASSWORD = 0x0020, ES_AUTOVSCROLL = 0x0040, ES_AUTOHSCROLL = 0x0080;
    const int ES_WANTRETURN = 0x1000, LBS_NOTIFY = 0x0001, WS_EX_CLIENTEDGE = 0x0200;
    const int WM_KEYDOWN = 0x0100, WM_SETFONT = 0x0030, VK_CONTROL = 0x11, VK_SHIFT = 0x10;

    static readonly int Command = RegisterWindowMessage("ShrimpRegressionTarget");

    readonly IntPtr[] fields = new IntPtr[4];
    int pasteKey;

    Target()
    {
        Text = "Shrimp Regression Target";
        ClientSize = new Size(640, 420);
        StartPosition = FormStartPosition.CenterScreen;
        Shown += (sender, args) =>
        {
            Activate();
            SetFocus(fields[0]);
        };
        Application.AddMessageFilter(this);
    }

    protected override void OnHandleCreated(EventArgs args)
    {
        base.OnHandleCreated(args);
        var child = WS_CHILD | WS_VISIBLE | WS_TABSTOP;
        fields[0] = Child("EDIT", "text", child | WS_VSCROLL | WS_HSCROLL | ES_MULTILINE | ES_AUTOVSCROLL | ES_AUTOHSCROLL | ES_WANTRETURN, 20);
        fields[1] = Child("EDIT", "field", child | ES_AUTOHSCROLL, 300);
        fields[2] = Child("EDIT", "secure", child | ES_AUTOHSCROLL | ES_PASSWORD, 340);
        fields[3] = Child("LISTBOX", "table", child | LBS_NOTIFY, 380);
    }

    IntPtr Child(string className, string name, int style, int top)
    {
        var height = className == "EDIT" && (style & ES_MULTILINE) != 0 ? 260 : 24;
        var hwnd = CreateWindowEx(WS_EX_CLIENTEDGE, className, "", style, 20, top, 600, height,
            Handle, new IntPtr(Array.IndexOf(new[] { "text", "field", "secure", "table" }, name) + 1),
            IntPtr.Zero, IntPtr.Zero);
        SendMessage(hwnd, WM_SETFONT, Font.ToHfont(), new IntPtr(1));
        return hwnd;
    }

    /// <summary>Notes how a paste reached a field: Ctrl+V or Shift+Insert.</summary>
    public bool PreFilterMessage(ref Message message)
    {
        if (message.Msg == WM_KEYDOWN && Array.IndexOf(fields, message.HWnd) >= 0)
        {
            var key = message.WParam.ToInt32();
            if (key == (int)Keys.V && GetKeyState(VK_CONTROL) < 0) pasteKey = 1;
            else if (key == (int)Keys.Insert && GetKeyState(VK_SHIFT) < 0) pasteKey = 2;
        }
        return false;
    }

    protected override void WndProc(ref Message message)
    {
        if (message.Msg != Command)
        {
            base.WndProc(ref message);
            return;
        }
        var command = message.WParam.ToInt32();
        if (command >= 1 && command <= fields.Length)
        {
            Activate();
            SetFocus(fields[command - 1]);
            message.Result = new IntPtr(1);
        }
        else if (command > 100 && command <= 100 + fields.Length)
        {
            message.Result = fields[command - 101];
        }
        else if (command == 105)
        {
            message.Result = new IntPtr(Array.IndexOf(fields, GetFocus()) + 1);
        }
        else if (command == 106)
        {
            message.Result = new IntPtr(pasteKey);
            pasteKey = 0;
        }
        else if (command == 9)
        {
            Close();
        }
    }

    [STAThread]
    static void Main()
    {
        Application.EnableVisualStyles();
        Application.Run(new Target());
    }
}
