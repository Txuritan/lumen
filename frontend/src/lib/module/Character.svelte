<script lang="ts">
    import Fieldset from '../component/Fieldset.svelte'
    import CharacterEntry from './CharacterEntry.svelte'
    import { state } from "../data"

    let name = ""

    const submit = async () => {
        let res = await fetch("/api/character/new?" + new URLSearchParams({ name: name }))
        if (res.status == 200) {
            $state = await res.json()
        }
    }
</script>

<Fieldset name={"Character"}>
    <ul>
        {#each $state.characters as character (character.name)}
        <CharacterEntry name={character.name}></CharacterEntry>
        {/each}
    </ul>
    <hr>
    <form method="get" on:submit|preventDefault={submit}>
        <input type="submit" value="add">
        <input type="text" name="name" placeholder="name" size="16" bind:value={name}>
    </form>
</Fieldset>
