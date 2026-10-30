/* 实验台交互：所有确认只写入本页模拟输入框。 */
'use strict';
const $ = id => document.getElementById(id);
const token = location.hash.slice(1);
let revision = 0, candidates = [], selected = 0, locked = [], english = '', translatedText = '';
let decodeTimer, translationTimer, holdTimer, pressedAt = null, held = false, waiting = null;
let decodeController, translationController, stopped = false;
let decoding = false, waitingDecode = null, lastAttempt = null;

async function api(path, body, signal) {
  const response = await fetch(path, {method:'POST', headers:{'Content-Type':'application/json','X-Lab-Token':token}, body:JSON.stringify(body), signal});
  const value = await response.json();
  if (!response.ok) {const error = new Error(value.error || '请求失败');error.retry=value.retry;throw error;}
  return value;
}
function current(){return candidates[selected];}
function setStatus(text,error=false){$('status').textContent=text;$('status').classList.toggle('error',error);}
function cancelWait(){waiting=null;waitingDecode=null;clearTimeout(holdTimer);if(pressedAt!==null)held=true;$('holdStatus').textContent='';}
function clearTranslation(){english='';translatedText='';clearTimeout(translationTimer);translationController?.abort();$('english').textContent='正在等待当前候选的翻译…';$('translationStatus').textContent='TranslateGemma 4B · 本机';}
function changed(){
  if(stopped)return;
  revision++;cancelWait();clearTranslation();decodeController?.abort();clearTimeout(decodeTimer);
  const codes=$('codes').value.toLowerCase();$('count').textContent=`${codes.length} / 96`;
  if(!codes.startsWith(locked.map(s=>s.code).join('')))locked=[];
  candidates=[];selected=0;decoding=!!codes;render();
  if(!codes){$('english').textContent='确定中文候选后，本地模型会自动翻译。';setStatus('等待输入');return;}
  setStatus('正在寻找字词边界…');decodeTimer=setTimeout(decode,65);
}
async function decode(){
  const own=revision;decodeController=new AbortController();
  try{
    const value=await api('/api/decode',{codes:$('codes').value,locked},decodeController.signal);
    if(own!==revision)return;
    candidates=value.candidates;selected=0;decoding=false;
    $('latency').textContent=`解码 ${value.elapsed_ms.toFixed(1)} ms`;
    setStatus(value.complete?`${candidates.length} 个整句候选`:value.message);
    render();scheduleTranslation();
    if(waitingDecode?.revision===own){const intent=waitingDecode;waitingDecode=null;commit(intent.bilingual);}
  }catch(error){if(error.name==='AbortError'||own!==revision)return;decoding=false;setStatus(error.message,true);render();}
}
function render(){
  const list=$('candidates');list.replaceChildren();
  if(!candidates.length){const empty=document.createElement('div');empty.className='empty';empty.textContent=decoding?'正在解码…':'输入连续五笔编码，查看自动切分与整句候选。';list.append(empty);}
  candidates.forEach((candidate,index)=>{
    const row=document.createElement('button');row.className='candidate'+(index===selected?' selected':'');row.setAttribute('aria-pressed',String(index===selected));
    const number=document.createElement('span');number.className='number';number.textContent=String(index+1).padStart(2,'0');
    const body=document.createElement('span');const word=document.createElement('span');word.className='word';word.textContent=candidate.text||'（继续输入）';
    const split=document.createElement('span');split.className='split';split.textContent=candidate.segments.map(s=>s.code).join(' | ');
    body.append(word,split);if(candidate.pending){const pending=document.createElement('span');pending.className='pending';pending.textContent='待完成：'+candidate.pending;body.append(pending);}
    row.append(number,body);row.onclick=()=>choose(index);list.append(row);
  });
  $('locked').hidden=!locked.length;$('lockedText').textContent='已固定：'+locked.map(s=>s.text).join('');
  const candidate=current();$('segmentsBox').hidden=!candidate;$('segments').replaceChildren();
  candidate?.segments.forEach((segment,index)=>{const button=document.createElement('button');button.title='固定到「'+segment.text+'」';const text=document.createElement('span');text.textContent=segment.text;const code=document.createElement('small');code.textContent=segment.code;button.append(text,code);button.onclick=()=>{locked=candidate.segments.slice(0,index+1);changed();$('codes').focus();};$('segments').append(button);});
  const disabled=!candidate||!!candidate.pending||decoding||stopped;
  $('commitChinese').disabled=disabled;$('commitBilingual').disabled=disabled;
}
function choose(index){if(index===selected)return;revision++;cancelWait();clearTranslation();selected=index;render();scheduleTranslation();}
function scheduleTranslation(force=false){clearTimeout(translationTimer);if(!$('autoTranslate').checked&&!force){$('english').textContent='自动翻译已暂停；长按确认时仍可请求翻译。';return;}if(current()&&!current().pending)translationTimer=setTimeout(()=>translate(revision),300);}
async function translate(own){
  const candidate=current();if(!candidate||candidate.pending||own!==revision||stopped)return;
  const text=candidate.text;translationController=new AbortController();$('translationStatus').textContent='本地翻译中… 首次加载可能需要数秒';
  try{
    const value=await api('/api/translate',{text},translationController.signal);
    if(own!==revision||current()?.text!==text)return;
    english=value.english;translatedText=text;$('english').textContent=english;$('translationStatus').textContent=value.cached?'已复用本次实验缓存':`翻译 ${(value.elapsed_ms/1000).toFixed(2)} 秒 · 本机`;
    if(waiting===own)commit(true);
  }catch(error){if(error.name==='AbortError'||own!==revision)return;if(error.retry){translationTimer=setTimeout(()=>translate(own),600);return;}$('english').textContent=error.message;$('translationStatus').textContent='中文仍可正常确认';cancelWait();}
}
function commit(bilingual){
  if(decoding&&!stopped){waitingDecode={revision,bilingual};$('holdStatus').textContent='等待解码完成，Esc 可取消';return;}
  const candidate=current();if(!candidate||candidate.pending||stopped){$('holdStatus').textContent='请先等待完整候选';return;}
  if(bilingual&&(!english||translatedText!==candidate.text)){waiting=revision;$('holdStatus').textContent='等待译文，Esc 可取消';scheduleTranslation(true);return;}
  const text=candidate.text+(bilingual?'\n'+english:'');
  lastAttempt={codes:$('codes').value,actual:candidate.text};
  $('output').value+=($('output').value?'\n\n':'')+text;
  $('codes').value='';locked=[];changed();$('holdStatus').textContent=bilingual?'双语已进入模拟输入框':'中文已进入模拟输入框';
}
$('codes').addEventListener('input',changed);
$('codes').addEventListener('compositionstart',()=>setStatus('当前是中文输入状态，请切到英文后输入编码',true));
$('codes').addEventListener('keydown',event=>{
  if(event.isComposing)return;
  if(event.code==='Space'){
    event.preventDefault();if(event.repeat||pressedAt!==null)return;pressedAt=performance.now();held=false;
    holdTimer=setTimeout(()=>{held=true;commit(true);},350);return;
  }
  if(event.key==='Escape'){event.preventDefault();if(waiting!==null||waitingDecode!==null||pressedAt!==null){cancelWait();held=true;}else{$('codes').value='';locked=[];changed();}return;}
  if(event.key==='ArrowDown'||event.key==='ArrowUp'){event.preventDefault();if(candidates.length)choose((selected+(event.key==='ArrowDown'?1:-1)+candidates.length)%candidates.length);return;}
  if(pressedAt!==null&&event.key.length===1&&!held){clearTimeout(holdTimer);held=true;commit(false);}
});
$('codes').addEventListener('keyup',event=>{if(event.code!=='Space')return;event.preventDefault();if(pressedAt===null)return;clearTimeout(holdTimer);const duration=performance.now()-pressedAt;pressedAt=null;if(!held)commit(duration>=350);held=false;});
function blur(){clearTimeout(holdTimer);pressedAt=null;held=false;cancelWait();}
window.addEventListener('blur',blur);document.addEventListener('visibilitychange',()=>{if(document.hidden)blur();});
$('codes').addEventListener('blur',()=>{if(pressedAt!==null)blur();});
$('commitChinese').onclick=()=>commit(false);$('commitBilingual').onclick=()=>commit(true);
$('retry').onclick=()=>{clearTranslation();scheduleTranslation(true);};
$('autoTranslate').onchange=()=>{cancelWait();clearTranslation();scheduleTranslation();};
$('reset').onclick=()=>{$('codes').value='';locked=[];changed();$('codes').focus();};
$('unlock').onclick=()=>{locked=[];changed();$('codes').focus();};
document.querySelectorAll('[data-example]').forEach(button=>button.onclick=()=>{locked=[];$('codes').value=button.dataset.example;changed();$('codes').focus();});
$('clearOutput').onclick=()=>{$('output').value='';};
$('copy').onclick=async()=>{try{await navigator.clipboard.writeText($('output').value);$('copyStatus').textContent='已复制';}catch{$('copyStatus').textContent='请选中输入框文字手动复制';}};
$('saveFeedback').onclick=async()=>{try{await api('/api/feedback',{codes:$('codes').value||lastAttempt?.codes||'',expected:$('expected').value,actual:current()?.text||lastAttempt?.actual||''});$('feedbackStatus').textContent='已保存到实验目录，未修改现用词库';}catch(error){$('feedbackStatus').textContent=error.message;}};
$('quit').onclick=async()=>{try{await api('/api/quit',{});}finally{stopped=true;revision++;blur();decodeController?.abort();translationController?.abort();clearTimeout(translationTimer);clearTimeout(decodeTimer);$('codes').disabled=true;setStatus('实验服务已关闭');render();}};
if(!token){stopped=true;setStatus('请从实验程序生成的入口打开页面',true);$('codes').disabled=true;}
render();
