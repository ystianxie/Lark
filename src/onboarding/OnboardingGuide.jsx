import React, {useEffect, useMemo, useState} from 'react';
import {invoke} from '@tauri-apps/api/core';
import clipboardImg from "../assets/clipboard.svg";
import todoImg from "../assets/todo.svg";
import weeklyReportImg from "../assets/weekly-report.svg";
import memoImg from "../assets/memo.svg";
import hostsImg from "../assets/component.svg";

const DEFAULT_SHORTCUTS = {
  hotkeyAwaken: 'Alt+Space',
  hotkeyClipboard: 'Ctrl+Alt+V',
  hotkeyFileJump: 'Ctrl+G',
};

function ShortcutKey({value}) {
  const keys = String(value || '')
    .split('+')
    .map((key) => key.trim())
    .filter(Boolean);

  return (
    <span className="onboardingGuide-shortcut" aria-label={value}>
      {keys.map((key, index) => (
        <React.Fragment key={key + '-' + index}>
          {index > 0 && <span className="onboardingGuide-shortcutPlus">+</span>}
          <kbd>{key}</kbd>
        </React.Fragment>
      ))}
    </span>
  );
}

function DemoSearch() {
  return (
    <div className="onboardingGuide-demoSearch" aria-label="主搜索窗口示意图">
      <div className="onboardingGuide-demoLabel">① 输入应用、文件或关键词</div>
      <div className="onboardingGuide-demoQuery">
        <span className="onboardingGuide-demoMark">L</span>
        <span>年度报告</span>
      </div>
      <div className="onboardingGuide-demoResult is-active">
        <span className="onboardingGuide-demoIcon is-file">文</span>
        <span><strong>年度报告.docx</strong><small>D:\Documents\工作</small></span>
        <em>Enter</em>
      </div>
      <div className="onboardingGuide-demoResult">
        <span className="onboardingGuide-demoIcon">应</span>
        <span><strong>打开应用</strong><small>也可以输入应用名称</small></span>
      </div>
      <div className="onboardingGuide-demoCaption">② 选择结果　③ 按 Enter 打开</div>
    </div>
  );
}

function WelcomePage({shortcut}) {
  return (
    <div className="onboardingGuide-page onboardingGuide-welcome">
      <div className="onboardingGuide-copy">
        <span className="onboardingGuide-eyebrow">第 1 步 · 基础操作</span>
        <h1>呼出、输入、执行</h1>
        <p>百灵鸟平时不会占用屏幕。需要时呼出主搜索窗口，输入内容并打开结果。</p>
        <div className="onboardingGuide-openPath">
          <div><span>打开百灵鸟</span><ShortcutKey value={shortcut} /></div>
          <b>→</b>
          <div><span>输入内容</span><strong>应用 · 文件 · 计算 · 网页</strong></div>
        </div>
        <p className="onboardingGuide-tryHint">你可以现在试一下；向导不会检测或限制你的操作。</p>
      </div>
      <DemoSearch />
    </div>
  );
}

function ResultActionsPage() {
  const results = [
    {icon: '文', name: '年度报告.docx', path: 'D:\\Documents\\工作', key: 'Alt + 1', active: true},
    {icon: '应', name: 'QQ.exe', path: 'Windows 应用', key: 'Alt + 2'},
    {icon: '文', name: '项目计划.xlsx', path: 'D:\\Project', key: 'Alt + 3'},
  ];

  return (
    <div className="onboardingGuide-page onboardingGuide-resultActionsPage">
      <div className="onboardingGuide-sectionHeading is-left">
        <span className="onboardingGuide-eyebrow">第 2 步 · 搜索技巧</span>
        <h1>文件和结果，可以更快操作</h1>
        <p>文件搜索既可以直接输入关键词，也可以在主搜索框为空时按 Tab 进入；先记住 Enter 即可。</p>
      </div>
      <div className="onboardingGuide-resultLayout">
        <div className="onboardingGuide-resultMock">
          <div className="onboardingGuide-fileMode">
            <ShortcutKey value="Tab" />
            <span><strong>直接进入文件搜索</strong><small>主搜索框为空时使用</small></span>
          </div>
          <div className="onboardingGuide-resultList">
            {results.map((result) => (
              <div className={'onboardingGuide-resultRow' + (result.active ? ' is-active' : '')} key={result.name}>
                <span className="onboardingGuide-resultIcon">{result.icon}</span>
                <span><strong>{result.name}</strong><small>{result.path}</small></span>
                <kbd>{result.key}</kbd>
              </div>
            ))}
          </div>
        </div>
        <div className="onboardingGuide-actionList">
          <article>
            <ShortcutKey value="Enter" />
            <span><strong>打开选中结果</strong><small>最常用的执行方式</small></span>
          </article>
          <article>
            <ShortcutKey value="Alt+Enter" />
            <span><strong>打开所在文件夹</strong><small>适用于应用和文件结果</small></span>
          </article>
          <article>
            <ShortcutKey value="Alt+1～9" />
            <span><strong>直接选择对应结果</strong><small>编号与当前结果列表对应</small></span>
          </article>
          <p>这些快捷操作只针对当前显示的搜索结果。</p>
        </div>
      </div>
    </div>
  );
}

function DirectAccessPage({shortcuts}) {
  return (
    <div className="onboardingGuide-page onboardingGuide-directPage">
      <div className="onboardingGuide-sectionHeading is-left">
        <span className="onboardingGuide-eyebrow">第 3 步 · 快捷入口</span>
        <h1>不打开主搜索，也能直接使用</h1>
        <p>剪贴板可以直接呼出；文件选择框则能快速跟到你刚刚浏览的文件夹。</p>
      </div>
      <div className="onboardingGuide-directGrid">
        <article className="onboardingGuide-clipboardCard">
          <span className="onboardingGuide-cardTag">剪贴板历史</span>
          <img src={clipboardImg} className="onboardingGuide-bigIcon"/>
          <ShortcutKey value={shortcuts.hotkeyClipboard} />
          <h2>找回之前复制的内容</h2>
          <p>直接查看最近复制的文本、图片和文件。</p>
        </article>
        <article className="onboardingGuide-jumpCard">
          <span className="onboardingGuide-cardTag">文件选择框跳转</span>
          <h2>把文件选择框带到刚才浏览的文件夹</h2>
          <div className="onboardingGuide-jumpFlow">
            <div><span>1</span><strong>资源管理器</strong><small>先查看目标文件夹</small></div>
            <b>→</b>
            <div><span>2</span><strong>打开 / 另存为</strong><small>切到文件选择框</small></div>
            <b>→</b>
            <div className="is-final"><span>3</span><ShortcutKey value={shortcuts.hotkeyFileJump} /><small>跳到刚才的文件夹</small></div>
          </div>
          <p className="onboardingGuide-note">百灵鸟记住的是最近一次处于前台的资源管理器目录。若还没有记录，请先切回资源管理器查看一次。</p>
        </article>
      </div>
    </div>
  );
}

function BuiltinsAndPluginsPage({shortcut}) {
  const builtins = [
    {icon: todoImg, title: '待办事项'},
    {icon: weeklyReportImg, title: '周报'},
    {icon: memoImg, title: '备忘录'},
    {icon: hostsImg, title: 'Hosts'},
  ];

  return (
    <div className="onboardingGuide-page onboardingGuide-toolsPage">
      <div className="onboardingGuide-sectionHeading is-left">
        <span className="onboardingGuide-eyebrow">第 4 步 · 工具与扩展</span>
        <h1>内置组件和插件扩展，不是一回事</h1>
        <p>内置组件可以直接搜索使用；组件库用于查看、配置和开发插件。</p>
      </div>
      <div className="onboardingGuide-toolsGrid">
        <article className="onboardingGuide-builtinsPanel">
          <div className="onboardingGuide-panelTitle">
            <span>开箱即用</span>
            <strong>内置组件</strong>
          </div>
          <div className="onboardingGuide-builtinsGrid">
            {builtins.map((item) => (
              <div key={item.title}><img src={item.icon} className="onboardingGuide-bigIcon"/><strong>{item.title}</strong></div>
            ))}
          </div>
          <div className="onboardingGuide-routeLine">
            <ShortcutKey value={shortcut} /><b>→</b><span>输入组件名称</span><b>→</b><ShortcutKey value="Enter" />
          </div>
        </article>
        <article className="onboardingGuide-pluginPanel">
          <div className="onboardingGuide-panelTitle">
            <span>插件管理与开发</span>
            <strong>组件库</strong>
          </div>
          <ul>
            <li>查看已安装的插件</li>
            <li>启用、停用和填写插件配置</li>
            <li>新增或编辑自己开发的插件</li>
          </ul>
          <div className="onboardingGuide-librarySearch">
            <span>打开方式</span>
            <strong>搜索“组件库”</strong>
            <ShortcutKey value="Enter" />
          </div>
        </article>
      </div>
      <p className="onboardingGuide-toolsFootnote">以后想重新查看本向导，可搜索“<strong>应用设置</strong>”，进入后点击“重新打开”。</p>
    </div>
  );
}

export default function OnboardingGuide() {
  const [pageIndex, setPageIndex] = useState(0);
  const [shortcuts, setShortcuts] = useState(DEFAULT_SHORTCUTS);
  const [isClosing, setIsClosing] = useState(false);
  const [error, setError] = useState('');

  useEffect(() => {
    let disposed = false;
    invoke('get_app_settings')
      .then((settings) => {
        if (!disposed) setShortcuts((current) => ({...current, ...settings}));
      })
      .catch(() => {
        // 保留默认展示值；读取失败不应阻止用户浏览或关闭向导。
      });
    return () => {
      disposed = true;
    };
  }, []);

  const pages = useMemo(() => [
    <WelcomePage key="welcome" shortcut={shortcuts.hotkeyAwaken} />,
    <ResultActionsPage key="result-actions" />,
    <DirectAccessPage key="direct-access" shortcuts={shortcuts} />,
    <BuiltinsAndPluginsPage key="builtins-plugins" shortcut={shortcuts.hotkeyAwaken} />,
  ], [shortcuts]);

  const nextLabels = ['下一步：搜索技巧', '下一步：快捷入口', '下一步：工具与扩展'];

  const complete = async () => {
    if (isClosing) return;
    setIsClosing(true);
    setError('');
    try {
      await invoke('complete_onboarding');
    } catch (reason) {
      setError(String(reason));
      setIsClosing(false);
    }
  };

  const isLastPage = pageIndex === pages.length - 1;

  return (
    <main className="onboardingGuide-shell">
      <header className="onboardingGuide-titlebar" data-tauri-drag-region>
        <div className="onboardingGuide-brand" data-tauri-drag-region>
          <span>L</span>
          <strong>百灵鸟新手向导</strong>
          <small>{pageIndex + 1} / {pages.length}</small>
        </div>
        <button className="onboardingGuide-close" type="button" aria-label="关闭新手向导"
                disabled={isClosing} onClick={complete}>×</button>
      </header>

      <section className="onboardingGuide-content">{pages[pageIndex]}</section>

      <footer className="onboardingGuide-footer">
        <div className="onboardingGuide-progress" aria-label={'第 ' + (pageIndex + 1) + ' 页，共 ' + pages.length + ' 页'}>
          {pages.map((_, index) => (
            <span className={index === pageIndex ? 'is-active' : ''} key={index} />
          ))}
        </div>
        <div className="onboardingGuide-actions">
          {error && <span className="onboardingGuide-error" role="alert">{error}</span>}
          {!isLastPage && (
            <button className="onboardingGuide-button is-quiet" type="button" disabled={isClosing}
                    onClick={complete}>跳过</button>
          )}
          {pageIndex > 0 && (
            <button className="onboardingGuide-button" type="button" disabled={isClosing}
                    onClick={() => setPageIndex((index) => index - 1)}>上一步</button>
          )}
          <button className="onboardingGuide-button is-primary" type="button" disabled={isClosing}
                  onClick={isLastPage ? complete : () => setPageIndex((index) => index + 1)}>
            {isLastPage ? (isClosing ? '正在完成…' : '开始使用') : nextLabels[pageIndex]}
          </button>
        </div>
      </footer>
    </main>
  );
}




