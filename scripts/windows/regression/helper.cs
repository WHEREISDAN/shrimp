// Test helper for the Windows regression walkthrough: synthetic keys and mouse
// buttons through SendInput (Shrimp's hooks treat them as the user's, because
// they lack Shrimp's own tag), the clipboard, the fields of the target app,
// Shrimp's tray icon, and the guard that stops a run as soon as the person at
// the PC uses the keyboard or mouse.
//
// run.ps1 builds it with the .NET Framework compiler that ships with Windows.
// It must run in the interactive desktop session; see docs/DEVELOPMENT.md.
//
// Environment: SHRIMP_REGRESSION_ABORT names the file that stops the run;
// SHRIMP_REGRESSION_ALLOWED lists the executables that may be in the
// foreground when keys are sent (default: the target app and Shrimp).

using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.IO;
using System.Linq;
using System.Runtime.InteropServices;
using System.Text;
using System.Threading;
using System.Web.Script.Serialization;
using System.Windows.Automation;

static class Helper
{
    const string TargetTitle = "Shrimp Regression Target";
    const string TargetExe = "ShrimpRegressionTarget.exe";
    const string ShrimpExe = "shrimp.exe";

    static readonly JavaScriptSerializer Json = new JavaScriptSerializer { MaxJsonLength = int.MaxValue };

    static void Output(object value)
    {
        Console.OutputEncoding = new UTF8Encoding(false);
        Console.WriteLine(Json.Serialize(value));
    }

    static int Fail(string message, int code = 1)
    {
        Console.Error.WriteLine(message);
        Environment.Exit(code);
        return code;
    }

    // MARK: Guard

    static string AbortFile { get { return Environment.GetEnvironmentVariable("SHRIMP_REGRESSION_ABORT"); } }

    static string[] Allowed
    {
        get
        {
            var list = Environment.GetEnvironmentVariable("SHRIMP_REGRESSION_ALLOWED");
            return string.IsNullOrEmpty(list) ? new[] { TargetExe, ShrimpExe } : list.Split(',');
        }
    }

    /// <summary>Keys this invocation pressed, released if it has to stop.</summary>
    static readonly List<Key> Pressed = new List<Key>();

    static string Aborted()
    {
        var file = AbortFile;
        return file != null && File.Exists(file) ? File.ReadAllText(file) : null;
    }

    static string Blocked()
    {
        var aborted = Aborted();
        if (aborted != null) return aborted;
        var exe = ForegroundExe();
        // The target also runs under other executable names, to look like other apps.
        if (!Allowed.Any(name => name.Equals(exe, StringComparison.OrdinalIgnoreCase)) && TitleOf(GetForegroundWindow()) != TargetTitle)
            return "the foreground app is " + (exe == "" ? "none" : exe);
        return null;
    }

    /// <summary>Stops before sending input when the run was stopped or another app is in front.</summary>
    static void Guard(string what)
    {
        var reason = Blocked();
        if (reason != null) Abort(reason + " (before " + what + ")");
    }

    static void Abort(string reason)
    {
        var held = Pressed.ToList();
        held.Reverse();
        // The inert key while Win is still down, so its release doesn't open Start.
        if (held.Any(key => key.Vk == VK_LWIN)) Mask();
        if (held.Count > 0) Send(held, false);
        Pressed.Clear();
        var file = AbortFile;
        if (file != null && !File.Exists(file)) File.WriteAllText(file, reason);
        Fail("aborted: " + reason, 3);
    }

    /// <summary>Sleeps, checking the guard every 20 ms.</summary>
    static void Wait(double seconds, string what)
    {
        var watch = Stopwatch.StartNew();
        while (watch.Elapsed.TotalSeconds < seconds)
        {
            var reason = Blocked();
            if (reason != null) Abort(reason + " (during " + what + ")");
            Thread.Sleep(Math.Max(1, Math.Min(20, (int)((seconds - watch.Elapsed.TotalSeconds) * 1000))));
        }
    }

    // MARK: Keys

    struct Key
    {
        public ushort Vk;
        public bool Extended;
        public int Mouse;
        public Key(ushort vk, bool extended = false) { Vk = vk; Extended = extended; Mouse = 0; }
        public static Key Button(int button) { return new Key { Mouse = button }; }
    }

    const ushort VK_LWIN = 0x5B;
    const ushort VK_ESCAPE = 0x1B;

    static readonly Dictionary<string, Key> Modifiers = new Dictionary<string, Key>
    {
        { "ctrl", new Key(0xA2) },
        { "win", new Key(VK_LWIN, true) },
        { "alt", new Key(0xA4) },
        { "shift", new Key(0xA0) },
    };

    static readonly Dictionary<string, Key> Keys = new Dictionary<string, Key>
    {
        { "space", new Key(0x20) }, { "esc", new Key(VK_ESCAPE) }, { "return", new Key(0x0D) },
        { "backspace", new Key(0x08) }, { "tab", new Key(0x09) }, { "insert", new Key(0x2D, true) },
        { "up", new Key(0x26, true) }, { "down", new Key(0x28, true) }, { "f13", new Key(0x7C) },
        { "middle", Key.Button(3) }, { "mouse4", Key.Button(4) }, { "mouse5", Key.Button(5) },
    };

    static Key Lookup(string name)
    {
        Key key;
        if (Modifiers.TryGetValue(name, out key) || Keys.TryGetValue(name, out key)) return key;
        if (name.Length == 1 && char.IsLetterOrDigit(name[0])) return new Key((ushort)char.ToUpperInvariant(name[0]));
        Fail("unknown key " + name);
        return key;
    }

    /// <summary>The keys of a combination such as "ctrl+win" or "shift+alt+z", in pressing order.</summary>
    static List<Key> Combo(string spec)
    {
        return spec.ToLowerInvariant().Split('+').Select(Lookup).ToList();
    }

    [StructLayout(LayoutKind.Sequential)]
    struct MOUSEINPUT { public int dx, dy; public uint mouseData, dwFlags, time; public IntPtr dwExtraInfo; }

    [StructLayout(LayoutKind.Sequential)]
    struct KEYBDINPUT { public ushort wVk, wScan; public uint dwFlags, time; public IntPtr dwExtraInfo; }

    [StructLayout(LayoutKind.Explicit)]
    struct INPUT
    {
        [FieldOffset(0)] public uint type;
        [FieldOffset(8)] public MOUSEINPUT mi;
        [FieldOffset(8)] public KEYBDINPUT ki;
    }

    [DllImport("user32.dll", SetLastError = true)]
    static extern uint SendInput(uint count, INPUT[] inputs, int size);

    [DllImport("user32.dll")]
    static extern uint MapVirtualKey(uint code, uint type);

    static INPUT Input(Key key, bool down)
    {
        var input = new INPUT();
        if (key.Mouse != 0)
        {
            input.type = 0;
            if (key.Mouse == 1) input.mi.dwFlags = down ? 0x0002u : 0x0004u;
            else if (key.Mouse == 2) input.mi.dwFlags = down ? 0x0008u : 0x0010u;
            else if (key.Mouse == 3) input.mi.dwFlags = down ? 0x0020u : 0x0040u;
            else
            {
                input.mi.dwFlags = down ? 0x0080u : 0x0100u;
                input.mi.mouseData = key.Mouse == 4 ? 1u : 2u;
            }
        }
        else
        {
            input.type = 1;
            input.ki.wVk = key.Vk;
            input.ki.wScan = (ushort)MapVirtualKey(key.Vk, 0);
            input.ki.dwFlags = (key.Extended ? 1u : 0u) | (down ? 0u : 2u);
        }
        return input;
    }

    /// <summary>Sends the keys in one batch, which no other input can come between.</summary>
    static void Send(IList<Key> keys, bool down)
    {
        var inputs = keys.Select(key => Input(key, down)).ToArray();
        if (SendInput((uint)inputs.Length, inputs, Marshal.SizeOf(typeof(INPUT))) != inputs.Length)
            Fail("SendInput failed: " + Marshal.GetLastWin32Error());
        foreach (var key in keys)
        {
            if (down) Pressed.Add(key);
            else Pressed.RemoveAll(other => other.Vk == key.Vk && other.Mouse == key.Mouse);
        }
        Thread.Sleep(8);
    }

    static void Send(Key key, bool down)
    {
        Send(new[] { key }, down);
    }

    /// <summary>An unassigned key, so a released Win key doesn't open Start when nothing else ran.</summary>
    static void Mask()
    {
        Send(new Key(0xE8), true);
        Send(new Key(0xE8), false);
    }

    static void Down(string spec)
    {
        Guard("pressing " + spec);
        foreach (var key in Combo(spec)) Send(key, true);
    }

    /// <summary>
    /// Releases in reverse order and at once, as Shrimp may paste the moment the
    /// shortcut ends; never guarded, so held keys always come back up.
    /// </summary>
    static void Up(string spec)
    {
        var keys = Combo(spec);
        keys.Reverse();
        Send(keys, false);
    }

    static void Hold(string spec, double seconds)
    {
        Down(spec);
        Wait(seconds, "holding " + spec);
        Up(spec);
    }

    static double Seconds(string text)
    {
        double value;
        if (!double.TryParse(text, System.Globalization.NumberStyles.Float, System.Globalization.CultureInfo.InvariantCulture, out value))
            Fail("not a number of seconds: " + text);
        return value;
    }

    // MARK: Windows and processes

    [DllImport("user32.dll")] static extern IntPtr GetForegroundWindow();
    [DllImport("user32.dll")] static extern uint GetWindowThreadProcessId(IntPtr hwnd, out uint pid);
    [DllImport("user32.dll", CharSet = CharSet.Unicode)] static extern IntPtr FindWindow(string cls, string title);
    [DllImport("user32.dll")] static extern bool SetForegroundWindow(IntPtr hwnd);
    [DllImport("user32.dll")] static extern bool BringWindowToTop(IntPtr hwnd);
    [DllImport("user32.dll")] static extern bool ShowWindow(IntPtr hwnd, int command);
    [DllImport("user32.dll")] static extern bool IsIconic(IntPtr hwnd);
    [DllImport("user32.dll")] static extern bool IsWindow(IntPtr hwnd);
    [DllImport("user32.dll")] static extern bool IsWindowVisible(IntPtr hwnd);
    [DllImport("user32.dll")] static extern bool AttachThreadInput(uint attach, uint to, bool on);
    [DllImport("kernel32.dll")] static extern uint GetCurrentThreadId();
    [DllImport("user32.dll", CharSet = CharSet.Unicode)] static extern int GetClassName(IntPtr hwnd, StringBuilder name, int size);
    [DllImport("user32.dll", CharSet = CharSet.Unicode)] static extern int GetWindowText(IntPtr hwnd, StringBuilder text, int size);

    delegate bool EnumProc(IntPtr hwnd, IntPtr data);
    [DllImport("user32.dll")] static extern bool EnumWindows(EnumProc proc, IntPtr data);

    static uint ProcessOf(IntPtr hwnd)
    {
        uint pid;
        GetWindowThreadProcessId(hwnd, out pid);
        return pid;
    }

    [DllImport("kernel32.dll")] static extern IntPtr OpenProcess(uint access, bool inherit, uint pid);
    [DllImport("kernel32.dll")] static extern bool CloseHandle(IntPtr handle);
    [DllImport("kernel32.dll", CharSet = CharSet.Unicode)] static extern bool QueryFullProcessImageName(IntPtr process, uint flags, StringBuilder name, ref uint size);

    static string ExeOf(uint pid)
    {
        var process = OpenProcess(0x1000, false, pid);
        if (process == IntPtr.Zero) return "";
        try
        {
            var name = new StringBuilder(1024);
            var size = (uint)name.Capacity;
            return QueryFullProcessImageName(process, 0, name, ref size) ? Path.GetFileName(name.ToString()) : "";
        }
        finally { CloseHandle(process); }
    }

    static string ForegroundExe()
    {
        var hwnd = GetForegroundWindow();
        return hwnd == IntPtr.Zero ? "" : ExeOf(ProcessOf(hwnd));
    }

    static string TitleOf(IntPtr hwnd)
    {
        var title = new StringBuilder(512);
        GetWindowText(hwnd, title, title.Capacity);
        return title.ToString();
    }

    static string ClassOf(IntPtr hwnd)
    {
        var name = new StringBuilder(256);
        GetClassName(hwnd, name, name.Capacity);
        return name.ToString();
    }

    static List<IntPtr> TopWindows(Func<IntPtr, bool> wanted)
    {
        var found = new List<IntPtr>();
        EnumWindows((hwnd, _) => { if (wanted(hwnd)) found.Add(hwnd); return true; }, IntPtr.Zero);
        return found;
    }

    /// <summary>Brings a window to the front, attaching to the foreground thread as Windows asks.</summary>
    static void Activate(IntPtr hwnd)
    {
        var deadline = Stopwatch.StartNew();
        while (GetForegroundWindow() != hwnd && deadline.ElapsedMilliseconds < 2000)
        {
            uint ignored;
            var foreground = GetWindowThreadProcessId(GetForegroundWindow(), out ignored);
            var current = GetCurrentThreadId();
            var attached = foreground != 0 && foreground != current && AttachThreadInput(current, foreground, true);
            if (IsIconic(hwnd)) ShowWindow(hwnd, 9);
            BringWindowToTop(hwnd);
            SetForegroundWindow(hwnd);
            if (attached) AttachThreadInput(current, foreground, false);
            Thread.Sleep(50);
        }
        if (GetForegroundWindow() != hwnd) Fail("could not bring the window to the front");
    }

    /// <summary>Brings the first visible window of an app the walkthrough started to the front.</summary>
    static void ActivateExe(string exe)
    {
        var window = TopWindows(hwnd => IsWindowVisible(hwnd) && TitleOf(hwnd) != ""
            && ExeOf(ProcessOf(hwnd)).Equals(exe, StringComparison.OrdinalIgnoreCase)).FirstOrDefault();
        if (window == IntPtr.Zero) Fail("no window of " + exe);
        Activate(window);
        Output(new Dictionary<string, object> { { "hwnd", window.ToInt64() } });
    }

    static void Frontmost()
    {
        var hwnd = GetForegroundWindow();
        var pid = hwnd == IntPtr.Zero ? 0 : ProcessOf(hwnd);
        Output(new Dictionary<string, object>
        {
            { "exe", pid == 0 ? "" : ExeOf(pid) }, { "pid", pid }, { "title", TitleOf(hwnd) },
            { "hwnd", hwnd.ToInt64() },
        });
    }

    // MARK: Target app

    [DllImport("user32.dll", CharSet = CharSet.Unicode)] static extern uint RegisterWindowMessage(string name);
    [DllImport("user32.dll")] static extern IntPtr SendMessageTimeout(IntPtr hwnd, uint msg, IntPtr wParam, IntPtr lParam, uint flags, uint timeout, out IntPtr result);
    [DllImport("user32.dll", EntryPoint = "SendMessageTimeoutW", CharSet = CharSet.Unicode)]
    static extern IntPtr SendMessageTimeoutText(IntPtr hwnd, uint msg, IntPtr wParam, StringBuilder text, uint flags, uint timeout, out IntPtr result);
    [DllImport("user32.dll", EntryPoint = "SendMessageTimeoutW", CharSet = CharSet.Unicode)]
    static extern IntPtr SendMessageTimeoutString(IntPtr hwnd, uint msg, IntPtr wParam, string text, uint flags, uint timeout, out IntPtr result);
    [DllImport("user32.dll")] static extern bool PostMessage(IntPtr hwnd, uint msg, IntPtr wParam, IntPtr lParam);
    // SendMessageTimeout gives EM_GETSEL's result wrong across processes; SendMessage marshals its pointers.
    [DllImport("user32.dll")] static extern IntPtr SendMessage(IntPtr hwnd, uint msg, out int start, out int end);

    const uint WM_SETTEXT = 0x000C, WM_GETTEXT = 0x000D, WM_GETTEXTLENGTH = 0x000E;
    const uint EM_GETSEL = 0x00B0, EM_SETSEL = 0x00B1, EM_GETLINECOUNT = 0x00BA;
    const uint SMTO_ABORTIFHUNG = 0x0002;

    /// <summary>The target's controls by name, as the target numbers them.</summary>
    static readonly string[] Fields = { "", "text", "field", "secure", "table" };

    static readonly uint TargetCommand = RegisterWindowMessage("ShrimpRegressionTarget");

    static IntPtr Target()
    {
        var hwnd = FindWindow(null, TargetTitle);
        if (hwnd == IntPtr.Zero) Fail("the target app isn't running");
        return hwnd;
    }

    static long Ask(IntPtr hwnd, uint message, long wParam, long lParam = 0)
    {
        IntPtr result;
        if (SendMessageTimeout(hwnd, message, new IntPtr(wParam), new IntPtr(lParam), SMTO_ABORTIFHUNG, 2000, out result) == IntPtr.Zero)
            Fail("the window didn't answer");
        return result.ToInt64();
    }

    /// <summary>The focused control of the target: its number and window.</summary>
    static KeyValuePair<int, IntPtr> FocusedField()
    {
        var target = Target();
        var index = (int)Ask(target, TargetCommand, 105);
        if (index < 1 || index >= Fields.Length) Fail("no field of the target is focused");
        return new KeyValuePair<int, IntPtr>(index, new IntPtr(Ask(target, TargetCommand, 100 + index)));
    }

    static string Text(IntPtr control)
    {
        var length = (int)Ask(control, WM_GETTEXTLENGTH, 0);
        var text = new StringBuilder(length + 1);
        IntPtr result;
        SendMessageTimeoutText(control, WM_GETTEXT, new IntPtr(length + 1), text, SMTO_ABORTIFHUNG, 2000, out result);
        return text.ToString();
    }

    /// <summary>Offsets in text whose line breaks are "\n" (what the walkthrough sees) to the control's "\r\n".</summary>
    static int ToControl(string text, int offset)
    {
        var lf = 0;
        for (var i = 0; i < text.Length && lf < offset; i++)
        {
            if (text[i] == '\r' && i + 1 < text.Length && text[i + 1] == '\n') continue;
            lf++;
            if (lf == offset) return i + 1;
        }
        return offset == 0 ? 0 : text.Length;
    }

    static int FromControl(string text, int offset)
    {
        var lf = 0;
        for (var i = 0; i < Math.Min(offset, text.Length); i++)
            if (!(text[i] == '\r' && i + 1 < text.Length && text[i + 1] == '\n')) lf++;
        return lf;
    }

    static void Focus(string name)
    {
        var target = Target();
        if (name == "quit")
        {
            PostMessage(target, TargetCommand, new IntPtr(9), IntPtr.Zero);
            Output(new Dictionary<string, object> { { "ok", true } });
            return;
        }
        var index = Array.IndexOf(Fields, name);
        if (index < 1) Fail("unknown field " + name);
        Activate(target);
        Ask(target, TargetCommand, index);
        var deadline = Stopwatch.StartNew();
        while (Ask(target, TargetCommand, 105) != index && deadline.ElapsedMilliseconds < 2000) Thread.Sleep(20);
        Output(new Dictionary<string, object> { { "ok", Ask(target, TargetCommand, 105) == index } });
    }

    static void PasteKey()
    {
        var key = Ask(Target(), TargetCommand, 106);
        Output(new Dictionary<string, object> { { "key", key == 1 ? "ctrl+v" : key == 2 ? "shift+insert" : null } });
    }

    static void ReadField()
    {
        var field = FocusedField();
        var text = Fields[field.Key] == "table" ? "" : Text(field.Value);
        int start, end;
        SendMessage(field.Value, EM_GETSEL, out start, out end);
        Output(new Dictionary<string, object>
        {
            { "field", Fields[field.Key] },
            { "value", text.Replace("\r\n", "\n") },
            // The lines the control shows: it breaks only at "\r\n".
            { "lines", Fields[field.Key] == "text" ? Ask(field.Value, EM_GETLINECOUNT, 0) : 1 },
            { "selection", new[] { FromControl(text, start), FromControl(text, end) - FromControl(text, start) } },
        });
    }

    static void SetField(string value)
    {
        var field = FocusedField();
        IntPtr result;
        SendMessageTimeoutString(field.Value, WM_SETTEXT, IntPtr.Zero, value.Replace("\r\n", "\n").Replace("\n", "\r\n"), SMTO_ABORTIFHUNG, 2000, out result);
        Output(new Dictionary<string, object> { { "ok", true } });
    }

    static void Select(int start, int length)
    {
        var field = FocusedField();
        var text = Text(field.Value);
        Ask(field.Value, EM_SETSEL, ToControl(text, start), ToControl(text, start + length));
        Output(new Dictionary<string, object> { { "ok", true } });
    }

    /// <summary>
    /// Holds a combination, then reports how long after its release the focused
    /// field's text changed (AUD-03), reading the field every 2 ms.
    /// </summary>
    static void Measure(string spec, double held, double timeout)
    {
        var control = FocusedField().Value;
        var before = Text(control);
        Down(spec);
        Wait(held, "holding " + spec);
        Up(spec);
        var released = Stopwatch.StartNew();
        while (released.Elapsed.TotalSeconds < timeout)
        {
            var value = Text(control);
            if (value != before)
            {
                Output(new Dictionary<string, object>
                {
                    { "ms", Math.Round(released.Elapsed.TotalMilliseconds, 1) },
                    { "value", value.Replace("\r\n", "\n") },
                });
                return;
            }
            Thread.Sleep(2);
        }
        Fail("the field didn't change within " + timeout + " s");
    }

    // MARK: Shrimp's tray icon

    const uint WM_CANCELMODE = 0x001F, WM_KEYDOWN = 0x0100;

    static uint ShrimpPid()
    {
        var process = Process.GetProcessesByName("shrimp").FirstOrDefault();
        if (process == null) Fail("Shrimp isn't running");
        return (uint)process.Id;
    }

    static IntPtr TrayWindow(uint pid)
    {
        var found = TopWindows(hwnd => ProcessOf(hwnd) == pid && ClassOf(hwnd) == "tray_icon_app");
        if (found.Count == 0) Fail("Shrimp has no tray icon window");
        return found[0];
    }

    [StructLayout(LayoutKind.Sequential)]
    struct NOTIFYICONIDENTIFIER { public uint cbSize; public IntPtr hWnd; public uint uID; public Guid guidItem; }

    [StructLayout(LayoutKind.Sequential)]
    struct RECT { public int left, top, right, bottom; }

    [DllImport("shell32.dll")] static extern int Shell_NotifyIconGetRect(ref NOTIFYICONIDENTIFIER icon, out RECT rect);

    /// <summary>
    /// Where Windows shows Shrimp's icon, if it shows it. tray-icon only
    /// handles a click when Windows can say this; for an icon hidden in the
    /// taskbar's overflow flyout it answers S_FALSE until the flyout is open,
    /// which is Explorer's.
    /// </summary>
    static bool IconRect(IntPtr tray, out RECT rect)
    {
        rect = new RECT();
        for (uint id = 0; id < 8; id++)
        {
            var icon = new NOTIFYICONIDENTIFIER { cbSize = (uint)Marshal.SizeOf(typeof(NOTIFYICONIDENTIFIER)), hWnd = tray, uID = id };
            var result = Shell_NotifyIconGetRect(ref icon, out rect);
            if (result == 0) return true;
            if (result == 1) return false;
        }
        return false;
    }

    // Native UI Automation: the .NET client has no proxy for the taskbar's
    // toolbars, so it sees no icons there. Only the methods used are named;
    // the others hold their places in the interfaces' tables.

    [ComImport, Guid("ff48dba4-60ef-4201-aa87-54103eef594e")]
    class CUIAutomation { }

    [ComImport, Guid("30cbe57d-d9d0-452a-ab13-7ac5ac4825ee"), InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]
    interface IUIAutomation
    {
        void CompareElements();
        void CompareRuntimeIds();
        void GetRootElement();
        [PreserveSig] int ElementFromHandle(IntPtr hwnd, out IUIAutomationElement element);
        void ElementFromPoint();
        void GetFocusedElement();
        void GetRootElementBuildCache();
        void ElementFromHandleBuildCache();
        void ElementFromPointBuildCache();
        void GetFocusedElementBuildCache();
        void CreateTreeWalker();
        void get_ControlViewWalker();
        void get_ContentViewWalker();
        [PreserveSig] int get_RawViewWalker(out IUIAutomationTreeWalker walker);
    }

    [ComImport, Guid("4042c624-389c-4afc-a630-9df854a541fc"), InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]
    interface IUIAutomationTreeWalker
    {
        void GetParentElement();
        [PreserveSig] int GetFirstChildElement(IUIAutomationElement element, out IUIAutomationElement first);
        void GetLastChildElement();
        [PreserveSig] int GetNextSiblingElement(IUIAutomationElement element, out IUIAutomationElement next);
    }

    [ComImport, Guid("d22108aa-8ac5-49a5-837b-37bbb3d7591e"), InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]
    interface IUIAutomationElement
    {
        void SetFocus();
        void GetRuntimeId();
        void FindFirst();
        void FindAll();
        void FindFirstBuildCache();
        void FindAllBuildCache();
        void BuildUpdatedCache();
        [PreserveSig] int GetCurrentPropertyValue(int property, [MarshalAs(UnmanagedType.Struct)] out object value);
    }

    const int UIA_BoundingRectanglePropertyId = 30001, UIA_NamePropertyId = 30005;

    /// <summary>The centre of the taskbar's button for Shrimp's icon (named by its tooltip), through UI Automation.</summary>
    static bool IconButton(out int x, out int y)
    {
        x = y = 0;
        var automation = (IUIAutomation)new CUIAutomation();
        IUIAutomationElement root;
        IUIAutomationTreeWalker walker;
        if (automation.ElementFromHandle(FindWindow("Shell_TrayWnd", null), out root) != 0 || root == null) return false;
        if (automation.get_RawViewWalker(out walker) != 0) return false;
        var pending = new Stack<IUIAutomationElement>();
        pending.Push(root);
        for (var seen = 0; pending.Count > 0 && seen < 1000; seen++)
        {
            var element = pending.Pop();
            object name, bounds;
            element.GetCurrentPropertyValue(UIA_NamePropertyId, out name);
            element.GetCurrentPropertyValue(UIA_BoundingRectanglePropertyId, out bounds);
            var box = bounds as double[];
            if (name is string && ((string)name).StartsWith("Shrimp") && box != null && box.Length == 4 && box[2] > 0 && box[3] > 0)
            {
                x = (int)(box[0] + box[2] / 2);
                y = (int)(box[1] + box[3] / 2);
                return true;
            }
            IUIAutomationElement child;
            if (walker.GetFirstChildElement(element, out child) != 0) continue;
            while (child != null)
            {
                pending.Push(child);
                IUIAutomationElement next;
                if (walker.GetNextSiblingElement(child, out next) != 0) break;
                child = next;
            }
        }
        return false;
    }

    [StructLayout(LayoutKind.Sequential)]
    struct POINT { public int x, y; }

    [DllImport("user32.dll")] static extern bool GetCursorPos(out POINT point);
    [DllImport("user32.dll")] static extern bool SetCursorPos(int x, int y);

    const int HiddenIcon = 4;

    /// <summary>
    /// Clicks Shrimp's taskbar icon as a person would, through Explorer:
    /// found with UI Automation, checked against where Windows says the icon
    /// is, and guarded like keys. The pointer goes back where it was. Exits
    /// with code 4 when the icon is hidden in the overflow flyout.
    /// </summary>
    static void ClickIcon(IntPtr tray, string button)
    {
        // Explorer places an icon a moment after a new instance adds it.
        RECT shown;
        var placed = Stopwatch.StartNew();
        while (!IconRect(tray, out shown))
        {
            if (placed.ElapsedMilliseconds > 5000) Fail("Shrimp's tray icon is hidden in the taskbar's overflow flyout", HiddenIcon);
            Thread.Sleep(250);
        }
        int x, y;
        if (!IconButton(out x, out y)) Fail("UI Automation found no taskbar button for Shrimp's icon");
        if (x < shown.left || x >= shown.right || y < shown.top || y >= shown.bottom)
            Fail(string.Format("UI Automation's button ({0}, {1}) isn't where Windows shows Shrimp's icon", x, y));
        Guard("clicking Shrimp's tray icon");
        POINT before;
        GetCursorPos(out before);
        SetCursorPos(x, y);
        Thread.Sleep(50);
        var press = new Key { Mouse = button == "right" ? 2 : 1 };
        Send(press, true);
        Send(press, false);
        Thread.Sleep(100);
        SetCursorPos(before.x, before.y);
    }

    static void TrayClick(string button)
    {
        ClickIcon(TrayWindow(ShrimpPid()), button);
        Output(new Dictionary<string, object> { { "ok", true } });
    }

    /// <summary>Opens the tray menu with a right click, lists its items, then invokes `item` or closes the menu.</summary>
    static void Tray(string item)
    {
        var pid = ShrimpPid();
        var tray = TrayWindow(pid);
        ClickIcon(tray, "right");
        IntPtr menu = IntPtr.Zero;
        var deadline = Stopwatch.StartNew();
        while (menu == IntPtr.Zero && deadline.ElapsedMilliseconds < 3000)
        {
            menu = TopWindows(hwnd => ClassOf(hwnd) == "#32768" && IsWindowVisible(hwnd) && ProcessOf(hwnd) == pid).FirstOrDefault();
            if (menu == IntPtr.Zero) Thread.Sleep(50);
        }
        if (menu == IntPtr.Zero) Fail("the tray menu didn't open");
        var element = AutomationElement.FromHandle(menu);
        var entries = element.FindAll(TreeScope.Descendants,
            new PropertyCondition(AutomationElement.ControlTypeProperty, ControlType.MenuItem));
        var listed = new List<Dictionary<string, object>>();
        AutomationElement chosen = null;
        foreach (AutomationElement entry in entries)
        {
            var name = entry.Current.Name;
            object toggle;
            var isChecked = entry.TryGetCurrentPattern(TogglePattern.Pattern, out toggle)
                && ((TogglePattern)toggle).Current.ToggleState == ToggleState.On;
            listed.Add(new Dictionary<string, object>
            {
                { "title", name }, { "enabled", entry.Current.IsEnabled }, { "checked", isChecked },
            });
            if (item != null && name == item) chosen = entry;
        }
        if (chosen != null)
        {
            ((InvokePattern)chosen.GetCurrentPattern(InvokePattern.Pattern)).Invoke();
        }
        else
        {
            PostMessage(tray, WM_CANCELMODE, IntPtr.Zero, IntPtr.Zero);
            Thread.Sleep(200);
            if (IsWindow(menu) && IsWindowVisible(menu)) PostMessage(menu, WM_KEYDOWN, new IntPtr(VK_ESCAPE), IntPtr.Zero);
        }
        Output(listed);
    }
    // MARK: Clipboard

    [DllImport("user32.dll", SetLastError = true)] static extern bool OpenClipboard(IntPtr owner);
    [DllImport("user32.dll")] static extern bool CloseClipboard();
    [DllImport("user32.dll")] static extern bool EmptyClipboard();
    [DllImport("user32.dll")] static extern uint EnumClipboardFormats(uint format);
    [DllImport("user32.dll")] static extern IntPtr GetClipboardData(uint format);
    [DllImport("user32.dll")] static extern IntPtr SetClipboardData(uint format, IntPtr memory);
    [DllImport("user32.dll", CharSet = CharSet.Unicode)] static extern uint RegisterClipboardFormat(string name);
    [DllImport("user32.dll", CharSet = CharSet.Unicode)] static extern int GetClipboardFormatName(uint format, StringBuilder name, int size);
    [DllImport("kernel32.dll")] static extern IntPtr GlobalAlloc(uint flags, UIntPtr bytes);
    [DllImport("kernel32.dll")] static extern IntPtr GlobalLock(IntPtr memory);
    [DllImport("kernel32.dll")] static extern bool GlobalUnlock(IntPtr memory);
    [DllImport("kernel32.dll")] static extern UIntPtr GlobalSize(IntPtr memory);
    [DllImport("kernel32.dll")] static extern IntPtr GlobalFree(IntPtr memory);

    const uint CF_UNICODETEXT = 13, CF_DIB = 8, CF_DIBV5 = 17;
    /// <summary>Formats holding GDI handles, which Windows makes again from the memory formats.</summary>
    static readonly uint[] HandleFormats = { 2, 3, 9, 14, 0x80, 0x82, 0x83, 0x8E };
    /// <summary>Formats Windows synthesises from CF_UNICODETEXT.</summary>
    static readonly uint[] TextCopies = { 1, 7, 16 };

    static void OpenOrFail()
    {
        for (var attempt = 0; attempt < 50; attempt++)
        {
            if (OpenClipboard(IntPtr.Zero)) return;
            Thread.Sleep(20);
        }
        Fail("another app is holding the clipboard");
    }

    static byte[] Read(IntPtr handle)
    {
        var size = (int)GlobalSize(handle).ToUInt64();
        var pointer = GlobalLock(handle);
        if (pointer == IntPtr.Zero) return null;
        var bytes = new byte[size];
        Marshal.Copy(pointer, bytes, 0, size);
        GlobalUnlock(handle);
        return bytes;
    }

    static void Put(uint format, byte[] bytes)
    {
        var memory = GlobalAlloc(0x0002, new UIntPtr((uint)Math.Max(1, bytes.Length)));
        var pointer = GlobalLock(memory);
        Marshal.Copy(bytes, 0, pointer, bytes.Length);
        GlobalUnlock(memory);
        if (SetClipboardData(format, memory) == IntPtr.Zero) GlobalFree(memory);
    }

    /// <summary>Keeps the helper's own writes out of clipboard history and cloud sync, as Shrimp does.</summary>
    static void MarkPrivate()
    {
        Put(RegisterClipboardFormat("ExcludeClipboardContentFromMonitorProcessing"), new byte[] { 0 });
        Put(RegisterClipboardFormat("CanIncludeInClipboardHistory"), BitConverter.GetBytes(0));
        Put(RegisterClipboardFormat("CanUploadToCloudClipboard"), BitConverter.GetBytes(0));
    }

    /// <summary>Every memory format with its name (registered formats get new numbers per session) as base64.</summary>
    static void ClipboardSave(string path)
    {
        var formats = new List<Dictionary<string, object>>();
        OpenOrFail();
        try
        {
            uint format = 0;
            var image = false;
            while ((format = EnumClipboardFormats(format)) != 0)
            {
                var isImage = format == CF_DIB || format == CF_DIBV5;
                if (HandleFormats.Contains(format) || TextCopies.Contains(format) || (format >= 0x300 && format <= 0x3FF) || (isImage && image))
                    continue;
                var handle = GetClipboardData(format);
                if (handle == IntPtr.Zero) continue;
                var bytes = Read(handle);
                if (bytes == null) continue;
                image |= isImage;
                var name = new StringBuilder(256);
                var named = format >= 0xC000 && GetClipboardFormatName(format, name, name.Capacity) > 0;
                formats.Add(new Dictionary<string, object>
                {
                    { "format", format }, { "name", named ? name.ToString() : null },
                    { "data", Convert.ToBase64String(bytes) },
                });
            }
        }
        finally { CloseClipboard(); }
        File.WriteAllText(path, Json.Serialize(formats));
        Output(new Dictionary<string, object> { { "formats", formats.Count } });
    }

    static void ClipboardRestore(string path)
    {
        var formats = Json.Deserialize<List<Dictionary<string, object>>>(File.ReadAllText(path));
        OpenOrFail();
        try
        {
            EmptyClipboard();
            foreach (var entry in formats)
            {
                var name = entry["name"] as string;
                var format = name != null ? RegisterClipboardFormat(name) : Convert.ToUInt32(entry["format"]);
                Put(format, Convert.FromBase64String((string)entry["data"]));
            }
            if (formats.Count > 0) MarkPrivate();
        }
        finally { CloseClipboard(); }
        Output(new Dictionary<string, object> { { "formats", formats.Count } });
    }

    static void ClipboardText()
    {
        OpenOrFail();
        try
        {
            var handle = GetClipboardData(CF_UNICODETEXT);
            var bytes = handle == IntPtr.Zero ? null : Read(handle);
            Output(bytes == null ? null : Encoding.Unicode.GetString(bytes).Split('\0')[0].Replace("\r\n", "\n"));
        }
        finally { CloseClipboard(); }
    }

    /// <summary>The names of the formats on the clipboard (registered ones by name).</summary>
    static void ClipboardFormats()
    {
        var names = new List<string>();
        OpenOrFail();
        try
        {
            uint format = 0;
            while ((format = EnumClipboardFormats(format)) != 0)
            {
                var name = new StringBuilder(256);
                names.Add(format >= 0xC000 && GetClipboardFormatName(format, name, name.Capacity) > 0 ? name.ToString() : format.ToString());
            }
        }
        finally { CloseClipboard(); }
        Output(names);
    }
    static void ClipboardSet(string text)
    {
        OpenOrFail();
        try
        {
            EmptyClipboard();
            Put(CF_UNICODETEXT, Encoding.Unicode.GetBytes(text + "\0"));
            MarkPrivate();
        }
        finally { CloseClipboard(); }
        Output(new Dictionary<string, object> { { "ok", true } });
    }

    // MARK: Speakers

    [ComImport, Guid("BCDE0395-E52F-467C-8E3D-C4579291692E")]
    class MMDeviceEnumerator { }

    [ComImport, Guid("A95664D2-9614-4F35-A746-DE8DB63617E6"), InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]
    interface IMMDeviceEnumerator
    {
        [PreserveSig] int EnumAudioEndpoints(int flow, int state, out IntPtr devices);
        [PreserveSig] int GetDefaultAudioEndpoint(int flow, int role, out IMMDevice device);
    }

    [ComImport, Guid("D666063F-1587-4E43-81F1-B948E807363F"), InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]
    interface IMMDevice
    {
        [PreserveSig] int Activate(ref Guid iid, int context, IntPtr parameters, [MarshalAs(UnmanagedType.IUnknown)] out object found);
    }

    [ComImport, Guid("5CDF2C82-841E-4546-9722-0CF74078229A"), InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]
    interface IAudioEndpointVolume
    {
        [PreserveSig] int RegisterControlChangeNotify(IntPtr notify);
        [PreserveSig] int UnregisterControlChangeNotify(IntPtr notify);
        [PreserveSig] int GetChannelCount(out uint count);
        [PreserveSig] int SetMasterVolumeLevel(float level, ref Guid context);
        [PreserveSig] int SetMasterVolumeLevelScalar(float level, ref Guid context);
        [PreserveSig] int GetMasterVolumeLevel(out float level);
        [PreserveSig] int GetMasterVolumeLevelScalar(out float level);
        [PreserveSig] int SetChannelVolumeLevel(uint channel, float level, ref Guid context);
        [PreserveSig] int SetChannelVolumeLevelScalar(uint channel, float level, ref Guid context);
        [PreserveSig] int GetChannelVolumeLevel(uint channel, out float level);
        [PreserveSig] int GetChannelVolumeLevelScalar(uint channel, out float level);
        [PreserveSig] int SetMute([MarshalAs(UnmanagedType.Bool)] bool mute, ref Guid context);
        [PreserveSig] int GetMute([MarshalAs(UnmanagedType.Bool)] out bool mute);
    }

    /// <summary>The default output device's volume control, where Shrimp's cues play.</summary>
    static IAudioEndpointVolume Speakers()
    {
        var enumerator = (IMMDeviceEnumerator)new MMDeviceEnumerator();
        IMMDevice device;
        if (enumerator.GetDefaultAudioEndpoint(0, 0, out device) != 0) Fail("no output device");
        var iid = typeof(IAudioEndpointVolume).GUID;
        object volume;
        if (device.Activate(ref iid, 23, IntPtr.Zero, out volume) != 0) Fail("no volume control");
        return (IAudioEndpointVolume)volume;
    }

    [ComImport, Guid("C02216F6-8C67-4B5B-9D00-D008E73E0064"), InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]
    interface IAudioMeterInformation
    {
        [PreserveSig] int GetPeakValue(out float peak);
    }

    /// <summary>The loudest the speakers played over `seconds`, 0 when nothing plays.</summary>
    static void OutputPeak(double seconds)
    {
        var enumerator = (IMMDeviceEnumerator)new MMDeviceEnumerator();
        IMMDevice device;
        if (enumerator.GetDefaultAudioEndpoint(0, 0, out device) != 0) Fail("no output device");
        var iid = typeof(IAudioMeterInformation).GUID;
        object meter;
        if (device.Activate(ref iid, 23, IntPtr.Zero, out meter) != 0) Fail("no meter");
        float loudest = 0;
        var watch = Stopwatch.StartNew();
        while (watch.Elapsed.TotalSeconds < seconds)
        {
            float peak;
            ((IAudioMeterInformation)meter).GetPeakValue(out peak);
            loudest = Math.Max(loudest, peak);
            Thread.Sleep(50);
        }
        Output(new Dictionary<string, object> { { "peak", loudest } });
    }

    /// <summary>Sets the speakers' mute and prints what it was, so it can be put back.</summary>
    static void OutputMute(bool mute)
    {
        var speakers = Speakers();
        bool was;
        speakers.GetMute(out was);
        var context = Guid.Empty;
        speakers.SetMute(mute, ref context);
        Output(new Dictionary<string, object> { { "muted", was } });
    }

    // MARK: The person at the PC

    delegate IntPtr HookProc(int code, IntPtr wParam, IntPtr lParam);
    [DllImport("user32.dll")] static extern IntPtr SetWindowsHookEx(int id, HookProc proc, IntPtr module, uint thread);
    [DllImport("user32.dll")] static extern IntPtr CallNextHookEx(IntPtr hook, int code, IntPtr wParam, IntPtr lParam);
    [DllImport("kernel32.dll", CharSet = CharSet.Unicode)] static extern IntPtr GetModuleHandle(string name);
    [DllImport("user32.dll")] static extern int GetMessage(out MSG message, IntPtr hwnd, uint min, uint max);

    [StructLayout(LayoutKind.Sequential)]
    struct MSG { public IntPtr hwnd; public uint message; public IntPtr wParam, lParam; public uint time; public int x, y; }

    static HookProc keyboardHook, mouseHook;

    /// <summary>
    /// Watches for input that wasn't injected (a person at the keyboard or
    /// mouse) and writes the abort file when it comes; the walkthrough and
    /// every helper call stop on that file. With a log file, it also writes
    /// down every injected key (time, key, up or down, and whether Shrimp
    /// sent it), to see what reached Windows. Runs until killed.
    /// </summary>
    static void Watch(string path, string log)
    {
        var clock = Stopwatch.StartNew();
        Func<string, HookProc> stopOn = kind => (code, wParam, lParam) =>
        {
            if (code >= 0)
            {
                // KBDLLHOOKSTRUCT.flags and MSLLHOOKSTRUCT.flags.
                var flags = (uint)Marshal.ReadInt32(lParam, kind == "key" ? 8 : 12);
                var injected = kind == "key" ? (flags & 0x10) != 0 : (flags & 0x01) != 0;
                if (!injected && !File.Exists(path))
                    File.WriteAllText(path, "the person at the PC used the " + (kind == "key" ? "keyboard" : "mouse"));
                if (injected && kind == "key" && log != null)
                {
                    var vk = Marshal.ReadInt32(lParam, 0);
                    var shrimp = Marshal.ReadInt64(lParam, 16) == 0x53485250;
                    var up = wParam.ToInt32() == 0x0101 || wParam.ToInt32() == 0x0105;
                    File.AppendAllText(log, string.Format("{0:F0} {1:X2} {2}{3}{4}",
                        clock.Elapsed.TotalMilliseconds, vk, up ? "up" : "down", shrimp ? " shrimp" : "", Environment.NewLine));
                }
            }
            return CallNextHookEx(IntPtr.Zero, code, wParam, lParam);
        };
        keyboardHook = stopOn("key");
        mouseHook = stopOn("mouse");
        var module = GetModuleHandle(null);
        if (SetWindowsHookEx(13, keyboardHook, module, 0) == IntPtr.Zero || SetWindowsHookEx(14, mouseHook, module, 0) == IntPtr.Zero)
            Fail("could not watch the input");
        Console.WriteLine("watching");
        MSG message;
        while (GetMessage(out message, IntPtr.Zero, 0, 0) > 0) { }
    }

    // MARK: Main

    static string Arg(string[] args, int index)
    {
        if (index >= args.Length) Fail(args[0] + ": missing argument");
        return args[index];
    }

    static int Main(string[] args)
    {
        if (args.Length == 0)
        {
            return Fail(string.Join("\n", new[]
            {
                "usage: shrimp-helper <command>",
                "  down <combo> | up <combo> | tap <combo> | hold <combo> <seconds>",
                "  clipboard-save <file> | clipboard-restore <file> | clipboard-text | clipboard-set <text> | clipboard-formats",
                "  frontmost | focus <text|field|secure|table|quit> | activate <hwnd> | activate-exe <exe>",
                "  target | field | set-field <text> | select <start> <length> | paste-key",
                "  measure <combo> <held> <timeout>",
                "  tray-click <left|right> | tray [item] | output-mute <true|false> | output-peak <seconds>",
                "  watch <abort file> [key log]",
            }));
        }
        switch (args[0])
        {
            case "down": Down(Arg(args, 1)); break;
            case "up": Up(Arg(args, 1)); break;
            case "tap":
                Down(Arg(args, 1));
                Wait(0.06, "tapping " + args[1]);
                Up(args[1]);
                break;
            case "hold": Hold(Arg(args, 1), Seconds(Arg(args, 2))); break;
            case "clipboard-save": ClipboardSave(Arg(args, 1)); break;
            case "clipboard-restore": ClipboardRestore(Arg(args, 1)); break;
            case "clipboard-text": ClipboardText(); break;
            case "clipboard-set": ClipboardSet(Arg(args, 1)); break;
            case "clipboard-formats": ClipboardFormats(); break;
            case "frontmost": Frontmost(); break;
            case "activate":
                var hwnd = new IntPtr(long.Parse(Arg(args, 1)));
                if (IsWindow(hwnd)) Activate(hwnd);
                Output(new Dictionary<string, object> { { "ok", IsWindow(hwnd) } });
                break;
            case "focus": Focus(Arg(args, 1)); break;
            case "activate-exe": ActivateExe(Arg(args, 1)); break;
            case "field": ReadField(); break;
            case "paste-key": PasteKey(); break;
            case "target":
                var target = FindWindow(null, TargetTitle);
                Output(new Dictionary<string, object> { { "running", target != IntPtr.Zero }, { "exe", target == IntPtr.Zero ? "" : ExeOf(ProcessOf(target)) } });
                break;
            case "set-field": SetField(Arg(args, 1)); break;
            case "select": Select(int.Parse(Arg(args, 1)), int.Parse(Arg(args, 2))); break;
            case "measure": Measure(Arg(args, 1), Seconds(Arg(args, 2)), Seconds(Arg(args, 3))); break;
            case "tray-click": TrayClick(Arg(args, 1)); break;
            case "tray": Tray(args.Length > 1 ? args[1] : null); break;
            case "output-mute": OutputMute(bool.Parse(Arg(args, 1))); break;
            case "output-peak": OutputPeak(Seconds(Arg(args, 1))); break;
            case "watch": Watch(Arg(args, 1), args.Length > 2 ? args[2] : null); break;
            default: return Fail("unknown command " + args[0]);
        }
        return 0;
    }
}
