import { useCallback, useEffect, useState } from "react";
import { api } from "../api/client";
import type { Photo, PhotoPayload } from "../types";

export function usePhotos(admin = false, enabled = true) {
  const [photos, setPhotos] = useState<Photo[]>([]);
  const [loading, setLoading] = useState(enabled);
  const [error, setError] = useState<string | null>(null);

  const reload = useCallback(async () => {
    if (!enabled) return;
    setLoading(true);
    setError(null);
    try {
      setPhotos(admin ? (await api.adminPhotos.list()).items : await api.publicPhotos());
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to load photos");
    } finally {
      setLoading(false);
    }
  }, [admin, enabled]);

  useEffect(() => {
    void reload();
  }, [reload]);

  const createPhoto = async (payload: PhotoPayload) => {
    const photo = await api.createPhoto(payload);
    await reload();
    return photo;
  };

  const updatePhoto = async (id: number, payload: PhotoPayload) => {
    const photo = await api.updatePhoto(id, payload);
    await reload();
    return photo;
  };

  const deletePhoto = async (id: number) => {
    await api.deletePhoto(id);
    await reload();
  };

  const updatePrivacy = async (id: number, privacy: Photo["privacy"]) => {
    const photo = await api.updatePrivacy(id, privacy);
    await reload();
    return photo;
  };

  const resetPhotos = async () => {
    const rows = await api.resetPhotos();
    setPhotos(rows);
    return rows;
  };

  return {
    photos,
    loading,
    error,
    reload,
    createPhoto,
    updatePhoto,
    deletePhoto,
    updatePrivacy,
    resetPhotos
  };
}
