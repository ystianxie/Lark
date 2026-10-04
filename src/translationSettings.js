export const TRANSLATION_SETTINGS_KEY = 'lark.translation.settings.v1';

export const TRANSLATION_PROVIDER_CATALOG = [
    {
        id: 'niutrans', name: '小牛翻译', category: 'translation', protocol: 'niutrans',
        endpoint: 'https://api.niutrans.com/NiuTransServer/translation',
        hint: 'HTTP 翻译服务，只需填写 API Key',
        fields: [
            {key: 'apiKey', label: 'API Key', type: 'password', required: true},
            {key: 'dictNo', label: '术语词典 ID', type: 'text'},
            {key: 'memoryNo', label: '翻译记忆 ID', type: 'text'},
            {key: 'dictflag', label: '启用词典', type: 'boolean', default: false},
            {key: 'dict', label: '句内词典 JSON', type: 'textarea', placeholder: '可选，例如 {"你好":"hello"}'}
        ],
        capabilities: {languageDetect: true, streaming: false},
    },
    {
        id: 'baidu', name: '百度翻译', category: 'translation', protocol: 'baidu-trans-vip',
        endpoint: 'https://fanyi-api.baidu.com/api/trans/vip/translate',
        hint: '通用翻译 API，需要 APP ID 和密钥',
        fields: [
            {key: 'appId', label: 'APP ID', type: 'text', required: true},
            {key: 'appKey', label: '密钥', type: 'password', required: true},
            {key: 'needIntervene', label: '启用我的术语库', type: 'boolean', default: false}
        ],
        capabilities: {languageDetect: true, streaming: false},
    },
    {
        id: 'tengxun', name: '腾讯翻译君', category: 'translation', protocol: 'tengxun',
        endpoint: 'https://tmt.tencentcloudapi.com',
        hint: '腾讯云机器翻译 TextTranslate，需要 SecretId 和 SecretKey',
        fields: [
            {key: 'secretId', label: 'SecretId', type: 'text', required: true},
            {key: 'secretKey', label: 'SecretKey', type: 'password', required: true},
            {key: 'region', label: '地域', type: 'text', default: 'ap-guangzhou'},
            {key: 'projectId', label: '项目 ID', type: 'text', default: '0'},
        ],
        capabilities: {languageDetect: true, streaming: false},
    },
    {
        id: 'deepseek',
        name: 'DeepSeek',
        category: 'translation',
        protocol: 'openai-compatible',
        kind: 'ai',
        hint: '直接调用 DeepSeek Chat Completions 接口；默认关闭 thinking 以降低翻译延迟',
        fields: [
            {key: 'apiKey', label: 'API Key', type: 'password', required: true},
            {
                key: 'model',
                label: '模型',
                type: 'text',
                default: 'deepseek-flash'
            },
            {key: 'baseUrl', label: 'Base URL', type: 'text', default: 'https://api.deepseek.com'}
        ]
    },
    {
        id:'zhipu',
        name:"智谱",
        category: "translation",
        protocol: 'openai-compatible',
        kind: 'ai',
        fields: [
            {key: 'apiKey', label: 'API Key', type: 'password', required: true},
            {
                key: 'model',
                label: '模型',
                type: 'text',
                default: 'glm-5.3'
            },
            {key: 'baseUrl', label: 'Base URL', type: 'text', default: 'https://open.bigmodel.cn/api/paas/v4'}
        ]
    },
    {id: 'openai-compatible', name: 'OpenAI 兼容接口', kind: 'ai', hint: '可填写自定义 Base URL'},
    {id: 'custom', name: '自定义服务', kind: 'custom', hint: '预留 Provider 接口'},
];

export const OCR_PROVIDER_CATALOG = [
    {
        id: 'ddddocr-rust',
        name: 'ddddocr-rust（远程）',
        category: 'ocr',
        protocol: 'ddddocr-rust',
        hint: '部署在其他服务器，只需填写服务地址',
        fields: [
            {
                key: 'endpoint',
                label: '服务地址',
                type: 'text',
                required: true,
                placeholder: '例如 http://192.168.1.10:8866/ocr'
            },
        ]
    },
    {
        id: 'paddleocr-local',
        name: 'PaddleOCR（本地）',
        category: 'ocr',
        protocol: 'local-http',
        hint: '本地运行，不上传截图',
        fields: [
            {key: 'endpoint', label: '服务地址', type: 'text', default: 'http://127.0.0.1:8866/ocr', required: true},
            {
                key: 'language',
                label: '识别语言',
                type: 'select',
                options: [{value: 'ch', label: '中文'}, {value: 'en', label: '英文'}, {
                    value: 'chinese_english',
                    label: '中英文'
                }],
                default: 'ch'
            }
        ]
    },
    {
        id: 'baidu-ocr',
        name: '百度 OCR',
        category: 'ocr',
        protocol: 'baidu-ocr',
        hint: '需要 APP ID、API Key 和 Secret Key',
        fields: [
            {key: 'appId', label: 'APP ID', type: 'text', required: true},
            {key: 'apiKey', label: 'API Key', type: 'text', required: true},
            {key: 'secretKey', label: 'Secret Key', type: 'password', required: true},
            {key: 'useAccurate', label: '使用高精度识别', type: 'boolean', default: false}
        ]
    }
];

export const DEFAULT_TRANSLATION_SETTINGS = {
    schemaVersion: 2,
    sourceLang: 'auto',
    simplifiedChineseTarget: 'en',
    otherLanguageTarget: 'zh-CN',
    requestTimeoutMs: 8000,
    autoClipboard: false,
    services: [],
    ocrServices: [],
};

export function readTranslationSettings() {
    try {
        const raw = localStorage.getItem(TRANSLATION_SETTINGS_KEY);
        if (!raw) return DEFAULT_TRANSLATION_SETTINGS;
        const parsed = JSON.parse(raw);
        return {
            ...DEFAULT_TRANSLATION_SETTINGS,
            ...parsed,
            services: Array.isArray(parsed.services) ? parsed.services : [],
            ocrServices: Array.isArray(parsed.ocrServices) ? parsed.ocrServices : [],
        };
    } catch {
        return DEFAULT_TRANSLATION_SETTINGS;
    }
}

export function normalizeTranslationSettings(value) {
    const next = {...DEFAULT_TRANSLATION_SETTINGS, ...value};
    next.requestTimeoutMs = Math.min(30000, Math.max(3000, Number(next.requestTimeoutMs) || 8000));
    const rawServices = Array.isArray(next.services) ? next.services : Object.entries(next.services || {}).map(([provider, value]) => ({
        ...value,
        provider
    }));
    next.services = rawServices.map((service, index) => {
        // 顶层字段来自当前设置表单，优先于旧版 config 快照，避免保存时恢复旧默认值。
        const source = {...(service.config || {}), ...service};
        return {
            id: source.id || `service-${Date.now()}-${index}`,
            provider: source.provider || 'deepseek',
            name: source.name || '',
            enabled: source.enabled !== false,
            apiKey: source.apiKey || '',
            appId: source.appId || '',
            appKey: source.appKey || '',
            secretId: source.secretId || '',
            secretKey: source.secretKey || '',
            region: source.region || (source.provider === 'tengxun' ? 'ap-guangzhou' : ''),
            projectId: source.projectId || (source.provider === 'tengxun' ? '0' : ''),
            baseUrl: source.baseUrl || (source.provider === 'deepseek'
                ? 'https://api.deepseek.com'
                : source.provider === 'zhipu'
                    ? 'https://open.bigmodel.cn/api/paas/v4'
                    : ''),
            model: source.model || (source.provider === 'deepseek' ? 'deepseek-flash' : source.provider === 'zhipu' ? 'glm-5.3' : ''),
            dictNo: source.dictNo || '',
            memoryNo: source.memoryNo || '',
            dictflag: source.dictflag === true,
            dict: source.dict || '',
            needIntervene: source.needIntervene === true,
            config: {...source},
        };
    });
    return next;
}
