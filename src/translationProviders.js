import {TRANSLATION_PROVIDER_CATALOG} from './translationSettings';

export function getTranslationProvider(providerId) {
    return TRANSLATION_PROVIDER_CATALOG.find(provider => provider.id === providerId) || null;
}

function encodeForm(values) {
    return new URLSearchParams(Object.entries(values).filter(([, value]) => value !== undefined && value !== null && value !== '')).toString();
}

export async function translateWithNiuTrans({text, sourceLang = 'auto', targetLang, service, signal}) {
    if (!service?.apiKey) throw new Error('小牛翻译未配置 API Key');
    if (!text?.trim()) throw new Error('翻译文本为空');
    const values = {
        from: sourceLang || 'auto', to: targetLang || 'zh', apikey: service.apiKey,
        src_text: text, dictNo: service.dictNo, memoryNo: service.memoryNo,
        dictflag: service.dictflag ? 1 : 0, dict: service.dict,
    };
    const body = encodeForm(values);
    const response = await fetch(getTranslationProvider('niutrans').endpoint, {
        method: body.length > 1500 ? 'POST' : 'GET',
        headers: body.length > 1500 ? {'Content-Type': 'application/x-www-form-urlencoded'} : undefined,
        body: body.length > 1500 ? body : undefined,
        signal: signal || undefined,
    });
    if (!response.ok) throw new Error(`小牛翻译请求失败（HTTP ${response.status}）`);
    const result = await response.json();
    if (result.error_code && String(result.error_code) !== '0') throw new Error(result.error_msg || `小牛翻译错误（${result.error_code}）`);
    return {text: result.tgt_text || '', sourceLang: result.from || sourceLang, targetLang: result.to || targetLang, raw: result};
}

function md5Hex(value) {
    // 百度签名应在宿主/Rust 层生成；这里明确拒绝在浏览器端实现，避免密钥进入前端请求链路。
    throw new Error('百度翻译签名需要由 Tauri/Rust Provider 生成');
}

export async function translateWithBaidu({text, sourceLang = 'auto', targetLang, service, signal}) {
    if (!service?.appId || !service?.appKey) throw new Error('百度翻译需要填写 APP ID 和密钥');
    if (!text?.trim()) throw new Error('翻译文本为空');
    const salt = `${Date.now()}${Math.floor(Math.random() * 1000)}`;
    const sign = md5Hex(`${service.appId}${text}${salt}${service.appKey}`);
    const body = new URLSearchParams({q: text, from: sourceLang || 'auto', to: targetLang, appid: service.appId, salt, sign, ...(service.needIntervene ? {needIntervene: '1'} : {})});
    const response = await fetch('https://fanyi-api.baidu.com/api/trans/vip/translate', {method: 'POST', headers: {'Content-Type': 'application/x-www-form-urlencoded'}, body, signal});
    if (!response.ok) throw new Error(`百度翻译请求失败（HTTP ${response.status}）`);
    const result = await response.json();
    if (result.error_code) throw new Error(result.error_msg || `百度翻译错误（${result.error_code}）`);
    return {text: Array.isArray(result.trans_result) ? result.trans_result.map(item => item.dst).join('\n') : '', sourceLang: result.from || sourceLang, targetLang: result.to || targetLang, raw: result};
}
