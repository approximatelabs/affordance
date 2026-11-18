// Source is data: a model tool call or an external `install` event supplies the same bytes.
// The parent owns revisions and controls; loaded code is an ordinary nested component.
import {loadModule} from 'affordance:compiler';
import {Tool} from '../lib/tool.tsx';

export default function LiveComponents() {
  const [state,setState]=useState({revisions:[],selected:-1}, 'source');
  const {revisions,selected}=state;
  const [error,setError]=useState('', 'source.error');

  /** Compile and select a component. Source must export a default component function. */
  function install({source}:{source:string}) {
    try {
      const module=loadModule(source);
      if (typeof module.default !== 'function') throw new Error('Expected a default component function');
      setState(current=>({revisions:[...current.revisions,module],selected:current.revisions.length}));
      setError('');
      return 'Installed';
    } catch(e) {
      setError(e.message);
      return e.message;
    }
  }
  /** Remove the selected component. Revision history is retained. */
  function remove({}: {}) {setState(s=>({...s,selected:-1}));}
  /** Select the previous source revision. Its local component state starts fresh. */
  function undo({}: {}) {setState(s=>({...s,selected:s.selected<0?s.revisions.length-1:s.selected-1}));}
  useEvent('install',install);
  useEvent('remove',remove);
  useEvent('undo',undo);

  const current=revisions[selected];
  let child=null;
  try {if(current) child=<current.default key={selected}/>;}
  catch(e) {child=<span>{`Component failed: ${e.message}`}</span>;}
  return <box>
    <Tool fn={install}>{'[install]'}</Tool>
    <Tool fn={remove}>{'[remove]'}</Tool>
    <Tool fn={undo}>{'[undo]'}</Tool>
    {error && <span>{error}</span>}
    {child}
  </box>;
}
