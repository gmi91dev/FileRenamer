namespace FileRenamer;

partial class Form1
{
    /// <summary>
    ///  Required designer variable.
    /// </summary>
    private System.ComponentModel.IContainer components = null;

    /// <summary>
    ///  Clean up any resources being used.
    /// </summary>
    /// <param name="disposing">true if managed resources should be disposed; otherwise, false.</param>
    protected override void Dispose(bool disposing)
    {
        if (disposing && (components != null))
        {
            components.Dispose();
        }
        base.Dispose(disposing);
    }

    #region Windows Form Designer generated code

    /// <summary>
    ///  Required method for Designer support - do not modify
    ///  the contents of this method with the code editor.
    /// </summary>
    private void InitializeComponent()
    {
        grpStep1 = new GroupBox();
        lblFolder = new Label();
        txtFolder = new TextBox();
        btnBrowse = new Button();
        lblType = new Label();
        chkVideo = new CheckBox();
        chkImage = new CheckBox();
        chkDoc = new CheckBox();
        btnGenerate = new Button();
        lblListHint = new Label();
        grpStep2 = new GroupBox();
        lblListFile = new Label();
        txtListFile = new TextBox();
        btnBrowseList = new Button();
        lblPrefix = new Label();
        txtPrefix = new TextBox();
        btnRename = new Button();
        lblRuleHint = new Label();
        btnVarsHelp = new Button();
        grpStep1.SuspendLayout();
        grpStep2.SuspendLayout();
        SuspendLayout();
        // 
        // grpStep1
        // 
        grpStep1.Location = new Point(12, 12);
        grpStep1.Name = "grpStep1";
        grpStep1.Size = new Size(456, 148);
        grpStep1.TabIndex = 0;
        grpStep1.TabStop = false;
        grpStep1.Text = "功能一：快速生成清单（快捷操作）";
        grpStep1.Controls.Add(lblFolder);
        grpStep1.Controls.Add(txtFolder);
        grpStep1.Controls.Add(btnBrowse);
        grpStep1.Controls.Add(lblType);
        grpStep1.Controls.Add(chkVideo);
        grpStep1.Controls.Add(chkImage);
        grpStep1.Controls.Add(chkDoc);
        grpStep1.Controls.Add(btnGenerate);
        grpStep1.Controls.Add(lblListHint);
        // 
        // lblFolder
        // 
        lblFolder.AutoSize = true;
        lblFolder.Location = new Point(18, 28);
        lblFolder.Name = "lblFolder";
        lblFolder.Size = new Size(54, 17);
        lblFolder.TabIndex = 0;
        lblFolder.Text = "文件夹：";
        // 
        // txtFolder
        // 
        txtFolder.Location = new Point(78, 25);
        txtFolder.Name = "txtFolder";
        txtFolder.ReadOnly = true;
        txtFolder.Size = new Size(286, 23);
        txtFolder.TabIndex = 1;
        // 
        // btnBrowse
        // 
        btnBrowse.Location = new Point(370, 24);
        btnBrowse.Name = "btnBrowse";
        btnBrowse.Size = new Size(72, 25);
        btnBrowse.TabIndex = 2;
        btnBrowse.Text = "浏览…";
        btnBrowse.UseVisualStyleBackColor = true;
        btnBrowse.Click += btnBrowse_Click;
        // 
        // lblType
        // 
        lblType.AutoSize = true;
        lblType.Location = new Point(18, 62);
        lblType.Name = "lblType";
        lblType.Size = new Size(173, 17);
        lblType.TabIndex = 3;
        lblType.Text = "文件类型（都不选 = 全部）：";
        // 
        // chkVideo
        // 
        chkVideo.AutoSize = true;
        chkVideo.Location = new Point(20, 88);
        chkVideo.Name = "chkVideo";
        chkVideo.Size = new Size(50, 21);
        chkVideo.TabIndex = 4;
        chkVideo.Text = "视频";
        chkVideo.UseVisualStyleBackColor = true;
        // 
        // chkImage
        // 
        chkImage.AutoSize = true;
        chkImage.Location = new Point(88, 88);
        chkImage.Name = "chkImage";
        chkImage.Size = new Size(50, 21);
        chkImage.TabIndex = 5;
        chkImage.Text = "图片";
        chkImage.UseVisualStyleBackColor = true;
        // 
        // chkDoc
        // 
        chkDoc.AutoSize = true;
        chkDoc.Location = new Point(156, 88);
        chkDoc.Name = "chkDoc";
        chkDoc.Size = new Size(50, 21);
        chkDoc.TabIndex = 6;
        chkDoc.Text = "文档";
        chkDoc.UseVisualStyleBackColor = true;
        // 
        // btnGenerate
        // 
        btnGenerate.Location = new Point(322, 84);
        btnGenerate.Name = "btnGenerate";
        btnGenerate.Size = new Size(120, 30);
        btnGenerate.TabIndex = 7;
        btnGenerate.Text = "生成清单表格";
        btnGenerate.UseVisualStyleBackColor = true;
        btnGenerate.Click += btnGenerate_Click;
        // 
        // lblListHint
        // 
        lblListHint.AutoSize = true;
        lblListHint.ForeColor = Color.DimGray;
        lblListHint.Location = new Point(18, 121);
        lblListHint.Name = "lblListHint";
        lblListHint.Size = new Size(300, 17);
        lblListHint.TabIndex = 8;
        lblListHint.Text = "生成位置：所选文件夹内的 重命名清单.xlsx";
        // 
        // grpStep2
        // 
        grpStep2.Location = new Point(12, 170);
        grpStep2.Name = "grpStep2";
        grpStep2.Size = new Size(456, 162);
        grpStep2.TabIndex = 9;
        grpStep2.TabStop = false;
        grpStep2.Text = "功能二：按清单重命名（独立功能）";
        grpStep2.Controls.Add(lblListFile);
        grpStep2.Controls.Add(txtListFile);
        grpStep2.Controls.Add(btnBrowseList);
        grpStep2.Controls.Add(lblPrefix);
        grpStep2.Controls.Add(txtPrefix);
        grpStep2.Controls.Add(btnRename);
        grpStep2.Controls.Add(lblRuleHint);
        grpStep2.Controls.Add(btnVarsHelp);
        // 
        // lblListFile
        // 
        lblListFile.AutoSize = true;
        lblListFile.Location = new Point(18, 30);
        lblListFile.Name = "lblListFile";
        lblListFile.Size = new Size(78, 17);
        lblListFile.TabIndex = 0;
        lblListFile.Text = "清单文件：";
        // 
        // txtListFile
        // 
        txtListFile.Location = new Point(102, 27);
        txtListFile.Name = "txtListFile";
        txtListFile.ReadOnly = true;
        txtListFile.Size = new Size(262, 23);
        txtListFile.TabIndex = 1;
        // 
        // btnBrowseList
        // 
        btnBrowseList.Location = new Point(370, 26);
        btnBrowseList.Name = "btnBrowseList";
        btnBrowseList.Size = new Size(72, 25);
        btnBrowseList.TabIndex = 2;
        btnBrowseList.Text = "浏览…";
        btnBrowseList.UseVisualStyleBackColor = true;
        btnBrowseList.Click += btnBrowseList_Click;
        // 
        // lblPrefix
        // 
        lblPrefix.AutoSize = true;
        lblPrefix.Location = new Point(18, 66);
        lblPrefix.Name = "lblPrefix";
        lblPrefix.Size = new Size(84, 17);
        lblPrefix.TabIndex = 3;
        lblPrefix.Text = "名称格式：";
        // 
        // txtPrefix
        // 
        txtPrefix.Location = new Point(108, 63);
        txtPrefix.Name = "txtPrefix";
        txtPrefix.Size = new Size(196, 23);
        txtPrefix.TabIndex = 4;
        txtPrefix.Text = "{new}.{ext}";
        // 
        // btnRename
        // 
        btnRename.Location = new Point(322, 61);
        btnRename.Name = "btnRename";
        btnRename.Size = new Size(120, 30);
        btnRename.TabIndex = 5;
        btnRename.Text = "读取清单并重命名";
        btnRename.UseVisualStyleBackColor = true;
        btnRename.Click += btnRename_Click;
        // 
        // lblRuleHint
        // 
        lblRuleHint.AutoSize = true;
        lblRuleHint.ForeColor = Color.DimGray;
        lblRuleHint.Location = new Point(18, 106);
        lblRuleHint.Name = "lblRuleHint";
        lblRuleHint.Size = new Size(180, 17);
        lblRuleHint.TabIndex = 6;
        lblRuleHint.Text = "表头即变量（详见右侧按钮）";
        // 
        // btnVarsHelp
        // 
        btnVarsHelp.Location = new Point(322, 99);
        btnVarsHelp.Name = "btnVarsHelp";
        btnVarsHelp.Size = new Size(120, 30);
        btnVarsHelp.TabIndex = 7;
        btnVarsHelp.Text = "查看变量说明";
        btnVarsHelp.UseVisualStyleBackColor = true;
        btnVarsHelp.Click += btnVarsHelp_Click;
        // 
        // Form1
        // 
        AutoScaleDimensions = new SizeF(7F, 17F);
        AutoScaleMode = AutoScaleMode.Font;
        AutoSize = true;
        AutoSizeMode = AutoSizeMode.GrowAndShrink;
        Controls.Add(grpStep2);
        Controls.Add(grpStep1);
        Font = new Font("Microsoft YaHei UI", 9F);
        FormBorderStyle = FormBorderStyle.FixedDialog;
        MaximizeBox = false;
        Name = "Form1";
        StartPosition = FormStartPosition.CenterScreen;
        Text = "FileRenamer";
        grpStep1.ResumeLayout(false);
        grpStep1.PerformLayout();
        grpStep2.ResumeLayout(false);
        grpStep2.PerformLayout();
        ResumeLayout(false);
    }

    #endregion

    private GroupBox grpStep1;
    private Label lblFolder;
    private TextBox txtFolder;
    private Button btnBrowse;
    private Label lblType;
    private CheckBox chkVideo;
    private CheckBox chkImage;
    private CheckBox chkDoc;
    private Button btnGenerate;
    private Label lblListHint;
    private GroupBox grpStep2;
    private Label lblListFile;
    private TextBox txtListFile;
    private Button btnBrowseList;
    private Label lblPrefix;
    private TextBox txtPrefix;
    private Button btnRename;
    private Label lblRuleHint;
    private Button btnVarsHelp;
}
