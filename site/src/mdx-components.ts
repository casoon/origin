import Diagram from './components/Diagram.astro';
import RepoFile from './components/RepoFile.astro';

// Components for docs/ MDX, which cannot import anything itself.
export const components = { Diagram, RepoFile };
