<script lang="ts">
    import { state, type Stat } from "../data"

    export let stat: Stat

    const submit = async () => {
        let res = await fetch(`/api/stat/remove/${encodeURI(stat.name)}`)
        if (res.status == 200) {
            $state = await res.json()
        }
    }
</script>

<form method="get" on:submit|preventDefault={submit}>
    <var>{stat.name}</var>
    {#if stat.type == "number"}
    <input type="button" value="number">
    {:else if stat.type == "boolean"}
    <input type="button" value="boolean">
    {/if}
    <input type="submit" value="remove">
</form>
