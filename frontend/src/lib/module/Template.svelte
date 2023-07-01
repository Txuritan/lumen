<script lang="ts">
    import Fieldset from '../component/Fieldset.svelte'
    import TemplateEntry from './TemplateEntry.svelte'
    import { state } from '../data'

    let name = ""
    let type = ""

    const submit = async () => {
        let res = await fetch("/api/stat/new?" + new URLSearchParams({ name: name, type: type }))
        if (res.status == 200) {
            $state = await res.json()
            name = ""
            type =""
        }
    }
</script>

<Fieldset name={"Template"}>
    <form method="get" on:submit|preventDefault={submit}>
        {#each $state.stats as stat (stat.name) }
        <TemplateEntry stat={stat}></TemplateEntry>
        <hr>
        {/each}
        <input type="submit" value="add">
        <input type="text" name="name" placeholder="name" size="16" bind:value={name}>
        <select name="type" bind:value={type}>
            <option value="number">number</option>
            <option value="boolean">boolean</option>
        </select>
    </form>
</Fieldset>
