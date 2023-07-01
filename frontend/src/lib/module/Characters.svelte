<script lang="ts">
    import Fieldset from "../component/Fieldset.svelte"
    import CharactersStat from "./CharactersStat.svelte"
    import type { Character } from "../data"

    export let character: Character
</script>

<Fieldset name={character.name}>
    <table>
        {#each character.stats as stat (stat.name)}
        <CharactersStat character={character} stat={stat}></CharactersStat>
        {/each}
    </table>
</Fieldset>
