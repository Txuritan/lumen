<script lang="ts">
    import Fieldset from '../component/Fieldset.svelte'
    import PartsEntry from './PartsEntry.svelte'
    import { state } from '../data'

    let name = ""
    let details = ""
    let part = ""
    let rarity = ""
    let company = ""

    const submitNew = async () => {
        let res = await fetch("/api/weapon/part/new?" + new URLSearchParams({ name: name, details: details, part: part, rarity: rarity, company: company }))
        if (res.status == 200) {
            $state = await res.json()
            name = ""
            details = ""
            part = ""
            rarity = ""
            company = ""
        }
    }

    const submitInit = async () => {
        let res = await fetch("/api/weapon/part/init")
        if (res.status == 200) {
            $state = await res.json()
        }
    }
</script>

<Fieldset name={"Parts"} open={false}>
    <ul>
        {#each $state.parts as part (part.name + part.type + part.rarity)}
        <PartsEntry part={part}></PartsEntry>
        {/each}
    </ul>
    <hr>
    <form method="get" on:submit|preventDefault={submitNew}>
        <input type="submit" value="add">
        <input type="text" name="name" placeholder="name" size="16" bind:value={name}>
        <input type="text" name="details" placeholder="details" size="16" bind:value={details}>
        <select name="part" bind:value={part}>
            <option value="body">body</option>
            <option value="barrel">barrel</option>
            <option value="magazine">magazine</option>
            <option value="stock">stock</option>
        </select>
        <select name="rarity" bind:value={rarity}>
            <option value="common">common</option>
            <option value="uncommon">uncommon</option>
            <option value="rare">rare</option>
            <option value="legendary">legendary</option>
            <option value="unique">unique</option>
        </select>
        <select name="company" bind:value={company}>
            <option value="arksys">Arksys Inc</option>
            <option value="dikarum">Dikarum & Sons</option>
            <option value="pecora">Pecora Group</option>
            <option value="sisterhood">Sisterhood of Blight</option>
            <option value="theia">Theia Manufacturing</option>
            <option value="west_field">West Field Mining Munitions</option>
        </select>
    </form>
    <br>
    <form method="get" on:submit|preventDefault={submitInit}>
        <input type="submit" value="init">
    </form>
</Fieldset>
