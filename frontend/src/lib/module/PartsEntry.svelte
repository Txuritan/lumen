<script lang="ts">
    import { state, type Part } from "../data"

    export let part: Part

    const submit = async () => {
        let res = await fetch(`/api/weapon/part/remove/${encodeURI(part.name)}`)
        if (res.status == 200) {
            $state = await res.json()
        }
    }
</script>

<li><var>{part.name}</var> - <var>{part.details}</var> <input type="button" value="{part.type}"><input type="button" value="{part.rarity }"><form method="get" on:submit|preventDefault={submit}><input type="submit" value="remove"></form></li>
