using System.Drawing.Drawing2D;

namespace FileRenamer;

/// <summary>
/// 旋转加载指示器（菊花）。WinForms 没有内置 spinner 控件，自绘一段圆弧并定时旋转。
/// 仅在 Visible 时启动动画，隐藏时停表避免空耗。
/// </summary>
public sealed class Spinner : Control
{
    // 用 UI 线程 Timer（非 System.Threading.Timer），Invalidate/OnPaint 天然在 UI 线程，无需 Invoke
    private readonly System.Windows.Forms.Timer timer = new();
    private float angle;

    public Spinner()
    {
        // AllPaintingInWmPaint + OptimizedDoubleBuffer：自绘控件标准样式，避免闪烁与背景擦除抖动
        SetStyle(ControlStyles.AllPaintingInWmPaint
            | ControlStyles.OptimizedDoubleBuffer
            | ControlStyles.UserPaint
            | ControlStyles.ResizeRedraw, true);
        Size = new Size(36, 36);
        TabStop = false;
        timer.Interval = 40; // 每 40ms 转 24°，约 600ms 一圈
        timer.Tick += (_, _) =>
        {
            angle = (angle + 24f) % 360f;
            Invalidate();
        };
    }

    protected override void OnVisibleChanged(EventArgs e)
    {
        base.OnVisibleChanged(e);
        timer.Enabled = Visible;
    }

    protected override void OnPaint(PaintEventArgs e)
    {
        base.OnPaint(e);
        e.Graphics.SmoothingMode = SmoothingMode.AntiAlias;
        float inset = 5f;
        var rect = new RectangleF(inset, inset, Width - inset * 2, Height - inset * 2);
        using var pen = new Pen(Color.SteelBlue, 3.5f)
        {
            StartCap = LineCap.Round,
            EndCap = LineCap.Round,
        };
        // 3/4 圆弧绕中心旋转，形成菊花样式
        e.Graphics.DrawArc(pen, rect, angle, 270f);
    }

    protected override void Dispose(bool disposing)
    {
        if (disposing) timer.Dispose();
        base.Dispose(disposing);
    }
}
